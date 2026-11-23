import { useCallback, useEffect, useRef, useState } from 'react';
import { createNode, createProject } from '../model/factory';
import { addNodeToFrame, removeNodeFromFrame } from '../model/mutate';
import type { Node, ProjectDoc } from '../model/types';
import {
  deleteProject as dbDeleteProject,
  listProjects,
  loadProject,
  saveProject,
} from '../storage/db';
import { deleteAssetBlob, saveAssetBlob } from '../storage/db';
import { useEditorStore } from '../store/editorStore';
import { useUiStore } from '../store/uiStore';
import { exportTokage, importTokage, downloadBlob } from '../lib/tokage';
import { invalidateAssetUrl } from '../three/assetUrls';
import {
  IconButtonShape, IconCopy, IconCube, IconDownload, IconLayers, IconMonitor,
  IconPencil, IconPlay, IconPlus, IconRect, IconRedo, IconSliders,
  IconSmartphone, IconTrash, IconType, IconUndo, IconUpload,
} from '../ui/icons';
import { btn, btnActive, divider, select as selectTheme } from '../ui/theme';

const LAST_PROJECT_KEY = 'tokagetype:lastProjectId';

/** UI モード切替（PC / スマホ）。手動選択は localStorage に保持される */
function UiModeSwitch() {
  const uiMode = useUiStore((s) => s.uiMode);
  const setUiMode = useUiStore((s) => s.setUiMode);
  const seg = (active: boolean): React.CSSProperties => ({
    display: 'inline-flex', alignItems: 'center', gap: 5,
    background: active ? 'var(--accent)' : 'transparent',
    color: active ? 'var(--accent-ink)' : 'var(--text-dim)',
    border: 'none', borderRadius: 3, padding: '4px 8px', cursor: 'pointer', fontSize: 12,
  });
  return (
    <span
      style={{
        display: 'inline-flex', gap: 2, padding: 2,
        border: '1px solid var(--line)', borderRadius: 'var(--radius)', background: 'var(--inset)',
      }}
      title="UI 切替（PC / スマホ）"
    >
      <button style={seg(uiMode === 'pc')} onClick={() => setUiMode('pc')}>
        <IconMonitor size={14} /> PC
      </button>
      <button style={seg(uiMode === 'mobile')} onClick={() => setUiMode('mobile')}>
        <IconSmartphone size={14} /> スマホ
      </button>
    </span>
  );
}

export default function Toolbar() {
  const doc = useEditorStore((s) => s.doc);
  const history = useEditorStore((s) => s.history);
  const mode = useEditorStore((s) => s.mode);
  const setMode = useEditorStore((s) => s.setMode);
  const saveState = useEditorStore((s) => s.saveState);
  const undo = useEditorStore((s) => s.undo);
  const redo = useEditorStore((s) => s.redo);
  const commit = useEditorStore((s) => s.commit);
  const loadDoc = useEditorStore((s) => s.loadDoc);
  const frame = useEditorStore((s) => s.currentFrame());
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);
  const selectNode = useEditorStore((s) => s.selectNode);
  const uiMode = useUiStore((s) => s.uiMode);

  const [projects, setProjects] = useState<ProjectDoc[]>([]);
  const importRef = useRef<HTMLInputElement>(null);

  const refreshProjects = useCallback(async () => {
    setProjects(await listProjects());
  }, []);

  useEffect(() => {
    void refreshProjects();
  }, [refreshProjects, doc?.id]);

  const switchProject = async (id: string) => {
    if (doc && id === doc.id) return;
    const target = await loadProject(id);
    if (target) {
      localStorage.setItem(LAST_PROJECT_KEY, id);
      loadDoc(target);
    }
  };

  const newProject = async () => {
    const p = createProject(`プロジェクト ${projects.length + 1}`);
    await saveProject(p);
    localStorage.setItem(LAST_PROJECT_KEY, p.id);
    loadDoc(p);
    await refreshProjects();
  };

  const deleteCurrentProject = async () => {
    if (!doc) return;
    if (!confirm(`プロジェクト「${doc.name}」を削除しますか？`)) return;
    for (const a of doc.assets) {
      await deleteAssetBlob(a.id);
      invalidateAssetUrl(a.id);
    }
    await dbDeleteProject(doc.id);
    const rest = await listProjects();
    if (rest.length > 0) {
      localStorage.setItem(LAST_PROJECT_KEY, rest[0].id);
      loadDoc(rest[0]);
    } else {
      const p = createProject('新規プロジェクト');
      await saveProject(p);
      localStorage.setItem(LAST_PROJECT_KEY, p.id);
      loadDoc(p);
    }
    await refreshProjects();
  };

  const addNode = (type: Node['type']) => {
    if (!doc || !frame) return;
    const offset = (frame.nodes.length % 5) * 18;
    const node = createNode(type, 40 + offset, 60 + offset);
    commit(addNodeToFrame(doc, frame.id, node));
    selectNode(node.id);
  };

  const deleteSelected = () => {
    if (!doc || !frame || !selectedNodeId) return;
    commit(removeNodeFromFrame(doc, frame.id, selectedNodeId));
    selectNode(null);
  };

  const duplicateSelected = () => {
    if (!doc || !frame || !selectedNodeId) return;
    const src = frame.nodes.find((n) => n.id === selectedNodeId);
    if (!src) return;
    const copy = JSON.parse(JSON.stringify(src)) as Node;
    copy.id = `${src.id}_copy_${Math.random().toString(36).slice(2, 7)}`;
    copy.name = `${src.name} のコピー`;
    copy.x += 24;
    copy.y += 24;
    if (copy.type === 'view3d') {
      // カメラIDは文書内で一意にする
      copy.cameras = copy.cameras.map((c) => ({ ...c, id: `${c.id}_c${Math.random().toString(36).slice(2, 6)}` }));
      const map = new Map<string, string>();
      (src as typeof copy).cameras.forEach((c, i) => map.set(c.id, copy.cameras[i].id));
      copy.activeCameraId = map.get(copy.activeCameraId) ?? copy.cameras[0]?.id ?? '';
      copy.actions = (copy.actions ?? []).map((a) =>
        a.type === 'setCamera' && a.viewNodeId === src.id
          ? { ...a, viewNodeId: copy.id, cameraId: map.get(a.cameraId) ?? a.cameraId }
          : a,
      );
    }
    commit(addNodeToFrame(doc, frame.id, copy));
    selectNode(copy.id);
  };

  const handleExport = async () => {
    if (!doc) return;
    const blob = await exportTokage(doc);
    downloadBlob(blob, `${doc.name}.tokage`);
  };

  const handleImport = async (file: File) => {
    try {
      const { doc: imported, blobs } = await importTokage(file);
      // ID が衝突する場合は新規IDで取り込む
      const existing = await loadProject(imported.id);
      const finalDoc = existing ? { ...imported, id: `${imported.id}_imp_${Date.now().toString(36)}` } : imported;
      for (const b of blobs) await saveAssetBlob(b.assetId, b.data);
      await saveProject(finalDoc);
      localStorage.setItem(LAST_PROJECT_KEY, finalDoc.id);
      loadDoc(finalDoc);
      await refreshProjects();
    } catch (e) {
      alert(`インポートに失敗しました: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  const canUndo = (history?.past.length ?? 0) > 0;
  const canRedo = (history?.future.length ?? 0) > 0;
  const editing = mode === 'edit';
  const mobile = uiMode === 'mobile';

  const modeToggle = (
    <span
      style={{
        display: 'inline-flex', gap: 2, padding: 2,
        border: '1px solid var(--line)', borderRadius: 'var(--radius)', background: 'var(--inset)',
      }}
    >
      <button
        style={{ ...btn, border: 'none', background: editing ? 'var(--accent)' : 'transparent', color: editing ? 'var(--accent-ink)' : 'var(--text-dim)', padding: '5px 10px' }}
        onClick={() => setMode('edit')}
      >
        <IconPencil size={14} /> 編集
      </button>
      <button
        style={{ ...btn, border: 'none', background: !editing ? 'var(--accent)' : 'transparent', color: !editing ? 'var(--accent-ink)' : 'var(--text-dim)', padding: '5px 10px' }}
        onClick={() => setMode('play')}
      >
        <IconPlay size={14} /> 再生
      </button>
    </span>
  );

  const saveIndicator = (
    <span style={{ color: 'var(--text-dim)', fontSize: 12, display: 'inline-flex', alignItems: 'center', gap: 6, whiteSpace: 'nowrap' }}>
      <span
        style={{
          width: 7, height: 7, borderRadius: '50%', display: 'inline-block',
          background: saveState === 'saved' ? 'var(--ok)' : 'var(--warn)',
        }}
      />
      {mobile ? '' : saveState === 'saved' ? '保存済み' : saveState === 'saving' ? '保存中…' : '未保存の変更'}
    </span>
  );

  if (mobile) {
    return (
      <div style={barStyle}>
        <span style={wordmarkStyle}>tokagetype</span>
        <select
          value={doc?.id ?? ''}
          onChange={(e) => void switchProject(e.target.value)}
          style={{ ...selectTheme, flex: '1 1 auto', minWidth: 0, maxWidth: 'none' }}
          title="プロジェクト切り替え"
        >
          {projects.map((p) => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
          {doc && !projects.some((p) => p.id === doc.id) && <option value={doc.id}>{doc.name}</option>}
        </select>
        {saveIndicator}
        <UiModeSwitch />
        {modeToggle}
        <input
          ref={importRef}
          type="file"
          accept=".tokage,.zip"
          style={{ display: 'none' }}
          onChange={(e) => {
            const f = e.target.files?.[0];
            e.target.value = '';
            if (f) void handleImport(f);
          }}
        />
      </div>
    );
  }

  return (
    <div style={barStyle}>
      <span style={wordmarkStyle}>tokagetype</span>

      <select
        value={doc?.id ?? ''}
        onChange={(e) => void switchProject(e.target.value)}
        style={{ ...selectTheme, maxWidth: 220 }}
        title="プロジェクト切り替え"
      >
        {projects.map((p) => (
          <option key={p.id} value={p.id}>{p.name}</option>
        ))}
        {doc && !projects.some((p) => p.id === doc.id) && <option value={doc.id}>{doc.name}</option>}
      </select>
      <button style={btn} onClick={() => void newProject()}><IconPlus size={14} /> 新規</button>
      <button style={btn} onClick={() => void deleteCurrentProject()}><IconTrash size={14} /> 削除</button>
      <button style={btn} onClick={() => void handleExport()}><IconDownload size={14} /> .tokage</button>
      <button style={btn} onClick={() => importRef.current?.click()}><IconUpload size={14} /> .tokage 読込</button>
      <input
        ref={importRef}
        type="file"
        accept=".tokage,.zip"
        style={{ display: 'none' }}
        onChange={(e) => {
          const f = e.target.files?.[0];
          e.target.value = '';
          if (f) void handleImport(f);
        }}
      />

      <span style={divider} />

      {editing && (
        <>
          <button style={btn} onClick={() => addNode('rect')}><IconRect size={14} /> 矩形</button>
          <button style={btn} onClick={() => addNode('text')}><IconType size={14} /> テキスト</button>
          <button style={btn} onClick={() => addNode('button')}><IconButtonShape size={14} /> ボタン</button>
          <button style={btn} onClick={() => addNode('view3d')}><IconCube size={14} /> 3Dビュー</button>
          <span style={divider} />
          <button style={btn} onClick={deleteSelected} disabled={!selectedNodeId}>削除</button>
          <button style={btn} onClick={duplicateSelected} disabled={!selectedNodeId}>複製</button>
          <button style={btn} onClick={undo} disabled={!canUndo}><IconUndo size={14} /> Undo</button>
          <button style={btn} onClick={redo} disabled={!canRedo}><IconRedo size={14} /> Redo</button>
        </>
      )}

      <span style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 10 }}>
        {saveIndicator}
        <UiModeSwitch />
        {modeToggle}
      </span>
    </div>
  );
}

/** スマホ UI の下部ツールバー（タップ対象 44px 以上、横スクロール可） */
export function MobileToolBar() {
  const doc = useEditorStore((s) => s.doc);
  const frame = useEditorStore((s) => s.currentFrame());
  const commit = useEditorStore((s) => s.commit);
  const selectNode = useEditorStore((s) => s.selectNode);
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);
  const undo = useEditorStore((s) => s.undo);
  const redo = useEditorStore((s) => s.redo);
  const history = useEditorStore((s) => s.history);
  const setSheet = useUiStore((s) => s.setSheet);
  const sheet = useUiStore((s) => s.sheet);

  const addNode = (type: Node['type']) => {
    if (!doc || !frame) return;
    const offset = (frame.nodes.length % 5) * 18;
    const node = createNode(type, 40 + offset, 60 + offset);
    commit(addNodeToFrame(doc, frame.id, node));
    selectNode(node.id);
  };
  const deleteSelected = () => {
    if (!doc || !frame || !selectedNodeId) return;
    commit(removeNodeFromFrame(doc, frame.id, selectedNodeId));
    selectNode(null);
  };
  const duplicateSelected = () => {
    if (!doc || !frame || !selectedNodeId) return;
    const src = frame.nodes.find((n) => n.id === selectedNodeId);
    if (!src) return;
    const copy = JSON.parse(JSON.stringify(src)) as Node;
    copy.id = `${src.id}_copy_${Math.random().toString(36).slice(2, 7)}`;
    copy.name = `${src.name} のコピー`;
    copy.x += 24;
    copy.y += 24;
    commit(addNodeToFrame(doc, frame.id, copy));
    selectNode(copy.id);
  };

  const toolBtn = (active = false): React.CSSProperties => ({
    display: 'inline-flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
    gap: 2, minWidth: 56, height: 52, padding: '4px 6px', flexShrink: 0,
    background: active ? 'var(--accent)' : 'var(--panel-2)',
    color: active ? 'var(--accent-ink)' : 'var(--text)',
    border: '1px solid var(--line)', borderRadius: 'var(--radius)',
    cursor: 'pointer', fontSize: 10,
  });

  return (
    <div
      style={{
        display: 'flex', gap: 6, padding: '6px 8px calc(6px + env(safe-area-inset-bottom))',
        background: 'var(--panel)', borderTop: '1px solid var(--line)',
        overflowX: 'auto', WebkitOverflowScrolling: 'touch', flexShrink: 0,
      }}
    >
      <button style={toolBtn(sheet === 'left')} onClick={() => setSheet(sheet === 'left' ? null : 'left')}>
        <IconLayers size={18} /> 画面/層
      </button>
      <button style={toolBtn()} onClick={() => addNode('rect')}><IconRect size={18} /> 矩形</button>
      <button style={toolBtn()} onClick={() => addNode('text')}><IconType size={18} /> テキスト</button>
      <button style={toolBtn()} onClick={() => addNode('button')}><IconButtonShape size={18} /> ボタン</button>
      <button style={toolBtn()} onClick={() => addNode('view3d')}><IconCube size={18} /> 3D</button>
      <button style={toolBtn()} onClick={duplicateSelected} disabled={!selectedNodeId}><IconCopy size={18} /> 複製</button>
      <button style={toolBtn()} onClick={deleteSelected} disabled={!selectedNodeId}><IconTrash size={18} /> 削除</button>
      <button style={toolBtn()} onClick={undo} disabled={(history?.past.length ?? 0) === 0}><IconUndo size={18} /> Undo</button>
      <button style={toolBtn()} onClick={redo} disabled={(history?.future.length ?? 0) === 0}><IconRedo size={18} /> Redo</button>
      <button style={toolBtn(sheet === 'right')} onClick={() => setSheet(sheet === 'right' ? null : 'right')}>
        <IconSliders size={18} /> プロパティ
      </button>
    </div>
  );
}

const barStyle: React.CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 8, padding: '8px 12px',
  background: 'var(--panel)', color: 'var(--text)', borderBottom: '1px solid var(--line)',
  flexWrap: 'wrap',
};
const wordmarkStyle: React.CSSProperties = {
  fontSize: 15, fontWeight: 800, letterSpacing: '0.02em', whiteSpace: 'nowrap',
  borderLeft: '4px solid var(--accent)', paddingLeft: 8, lineHeight: 1.1,
};
