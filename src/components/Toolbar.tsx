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
import { exportTokage, importTokage, downloadBlob } from '../lib/tokage';
import { invalidateAssetUrl } from '../three/assetUrls';

const LAST_PROJECT_KEY = 'tokagetype:lastProjectId';

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

  return (
    <div style={barStyle}>
      <strong style={{ fontSize: 15 }}>🦎 tokagetype</strong>

      <select
        value={doc?.id ?? ''}
        onChange={(e) => void switchProject(e.target.value)}
        style={selectStyle}
        title="プロジェクト切り替え"
      >
        {projects.map((p) => (
          <option key={p.id} value={p.id}>{p.name}</option>
        ))}
        {doc && !projects.some((p) => p.id === doc.id) && <option value={doc.id}>{doc.name}</option>}
      </select>
      <button style={btnStyle} onClick={() => void newProject()}>＋ 新規</button>
      <button style={btnStyle} onClick={() => void deleteCurrentProject()}>🗑 削除</button>
      <button style={btnStyle} onClick={() => void handleExport()}>⬇ .tokage</button>
      <button style={btnStyle} onClick={() => importRef.current?.click()}>⬆ .tokage 読込</button>
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

      <span style={dividerStyle} />

      {editing && (
        <>
          <button style={btnStyle} onClick={() => addNode('rect')}>▭ 矩形</button>
          <button style={btnStyle} onClick={() => addNode('text')}>T テキスト</button>
          <button style={btnStyle} onClick={() => addNode('button')}>🔘 ボタン</button>
          <button style={btnStyle} onClick={() => addNode('view3d')}>🧊 3Dビュー</button>
          <span style={dividerStyle} />
          <button style={btnStyle} onClick={deleteSelected} disabled={!selectedNodeId}>削除</button>
          <button style={btnStyle} onClick={duplicateSelected} disabled={!selectedNodeId}>複製</button>
          <button style={btnStyle} onClick={undo} disabled={!canUndo}>↩ Undo</button>
          <button style={btnStyle} onClick={redo} disabled={!canRedo}>↪ Redo</button>
        </>
      )}

      <span style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ color: '#8ea6c8', fontSize: 12 }}>
          {saveState === 'saved' ? '✓ 保存済み' : saveState === 'saving' ? '保存中…' : '● 未保存の変更'}
        </span>
        <button
          style={{ ...btnStyle, ...(editing ? {} : modeBtnActiveStyle) }}
          onClick={() => setMode('edit')}
        >
          ✎ 編集
        </button>
        <button
          style={{ ...btnStyle, ...(editing ? modeBtnActiveStyle : {}) }}
          onClick={() => setMode('play')}
        >
          ▶ 再生
        </button>
      </span>
    </div>
  );
}

const barStyle: React.CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 8, padding: '8px 12px',
  background: '#141c2c', color: '#eef3fa', borderBottom: '1px solid #0c1119',
  flexWrap: 'wrap',
};
const btnStyle: React.CSSProperties = {
  background: '#243350', color: '#e6eefb', border: '1px solid #3a4f7a',
  borderRadius: 6, padding: '5px 10px', cursor: 'pointer', fontSize: 13,
};
const modeBtnActiveStyle: React.CSSProperties = { background: '#2f6fd0', borderColor: '#2f6fd0', color: '#fff' };
const selectStyle: React.CSSProperties = {
  background: '#0f1626', color: '#e6eefb', border: '1px solid #3a4f7a',
  borderRadius: 6, padding: '5px 8px', fontSize: 13, maxWidth: 220,
};
const dividerStyle: React.CSSProperties = { width: 1, height: 22, background: '#3a4f7a', margin: '0 2px' };
