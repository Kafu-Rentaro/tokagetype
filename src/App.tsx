import { useEffect, useRef, useState } from 'react';
import KonvaCanvas from './components/KonvaCanvas';
import Panels from './components/Panels';
import PlayView from './components/PlayView';
import PropertyPanel from './components/PropertyPanel';
import Toolbar from './components/Toolbar';
import SculptView from './sculpt/SculptView';
import { removeNodeFromFrame } from './model/mutate';
import { buildSampleProject } from './sample';
import { listProjects, loadProject, saveAssetBlob, saveProject } from './storage/db';
import { useEditorStore } from './store/editorStore';
import type { View3DNode } from './model/types';

const LAST_PROJECT_KEY = 'tokagetype:lastProjectId';
let bootstrapStarted = false;

export default function App() {
  const doc = useEditorStore((s) => s.doc);
  const mode = useEditorStore((s) => s.mode);
  const loadDoc = useEditorStore((s) => s.loadDoc);
  const sculptNodeId = useEditorStore((s) => s.sculptNodeId);
  const setSculptNode = useEditorStore((s) => s.setSculptNode);
  const [ready, setReady] = useState(false);
  const saveTimer = useRef<number | null>(null);

  // 初回起動: 保存済みプロジェクトがあれば開き、無ければサンプルを用意する
  useEffect(() => {
    if (bootstrapStarted) {
      setReady(true);
      return;
    }
    bootstrapStarted = true;
    let cancelled = false;
    (async () => {
      try {
        const projects = await listProjects();
        if (projects.length === 0) {
          const sample = await buildSampleProject();
          await saveProject(sample.doc);
          for (const b of sample.blobs) await saveAssetBlob(b.assetId, b.data);
          localStorage.setItem(LAST_PROJECT_KEY, sample.doc.id);
          if (!cancelled) loadDoc(sample.doc);
        } else {
          const lastId = localStorage.getItem(LAST_PROJECT_KEY);
          const target = (lastId && (await loadProject(lastId))) || projects[0];
          if (!cancelled) loadDoc(target);
        }
      } catch (e) {
        console.error('初期化に失敗しました', e);
      } finally {
        if (!cancelled) setReady(true);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 自動保存: 編集後 500ms デバウンス（SPEC「保存」）
  useEffect(() => {
    if (!doc) return;
    const { saveState, setSaveState } = useEditorStore.getState();
    if (saveState === 'saved') return;
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      setSaveState('saving');
      void saveProject(doc).then(() => {
        if (useEditorStore.getState().doc === doc) setSaveState('saved');
        else useEditorStore.getState().setSaveState('dirty');
      });
    }, 500);
    return () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
    };
  }, [doc]);

  // キーボードショートカット: Undo/Redo, Delete
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const isFormField = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT' || target.isContentEditable;
      if (isFormField) return;
      const state = useEditorStore.getState();
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        if (e.shiftKey) state.redo();
        else state.undo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
        e.preventDefault();
        state.redo();
      } else if ((e.key === 'Delete' || e.key === 'Backspace') && state.selectedNodeId && state.mode === 'edit') {
        const frame = state.currentFrame();
        const d = state.doc;
        if (frame && d) {
          state.commit(removeNodeFromFrame(d, frame.id, state.selectedNodeId));
          state.selectNode(null);
        }
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  const sculptNode = (() => {
    if (!doc || !sculptNodeId) return null;
    for (const f of doc.frames) {
      const n = f.nodes.find((node) => node.id === sculptNodeId);
      if (n && n.type === 'view3d') return n as View3DNode;
    }
    return null;
  })();

  if (!ready) {
    return (
      <div style={{ height: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#141c2c', color: '#dbe6f5' }}>
        tokagetype を読み込み中…
      </div>
    );
  }

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column', background: '#141c2c' }}>
      <Toolbar />
      {mode === 'edit' ? (
        <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
          <Panels />
          <div style={{ flex: 1, minWidth: 0 }}>
            <KonvaCanvas />
          </div>
          <PropertyPanel />
        </div>
      ) : (
        <PlayView />
      )}
      {sculptNode && <SculptView node={sculptNode} onClose={() => setSculptNode(null)} />}
    </div>
  );
}
