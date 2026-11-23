import { createFrame } from '../model/factory';
import { cloneDoc, moveNodeLayer, removeNodeFromFrame } from '../model/mutate';
import { useEditorStore } from '../store/editorStore';

export default function Panels() {
  const doc = useEditorStore((s) => s.doc);
  const frame = useEditorStore((s) => s.currentFrame());
  const currentFrameId = useEditorStore((s) => s.currentFrameId);
  const setCurrentFrame = useEditorStore((s) => s.setCurrentFrame);
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);
  const selectNode = useEditorStore((s) => s.selectNode);
  const commit = useEditorStore((s) => s.commit);
  const patchNode = useEditorStore((s) => s.patchNode);

  if (!doc || !frame) return <div style={panelStyle} />;

  const addFrame = () => {
    const f = createFrame(`フレーム ${doc.frames.length + 1}`);
    const next = cloneDoc(doc);
    next.frames.push(f);
    commit(next);
    setCurrentFrame(f.id);
  };

  const deleteFrame = (frameId: string) => {
    if (doc.frames.length <= 1) return;
    const next = cloneDoc(doc);
    next.frames = next.frames.filter((f) => f.id !== frameId);
    if (next.startFrameId === frameId) next.startFrameId = next.frames[0].id;
    commit(next);
    if (currentFrameId === frameId) setCurrentFrame(next.frames[0].id);
  };

  const setStartFrame = (frameId: string) => {
    const next = cloneDoc(doc);
    next.startFrameId = frameId;
    commit(next);
  };

  return (
    <div style={panelStyle}>
      <div style={sectionStyle}>
        <div style={sectionHeaderStyle}>
          <span>フレーム</span>
          <button style={smallBtnStyle} onClick={addFrame}>＋ 追加</button>
        </div>
        {doc.frames.map((f) => (
          <div
            key={f.id}
            onClick={() => setCurrentFrame(f.id)}
            style={{
              ...rowStyle,
              ...(f.id === frame.id ? rowActiveStyle : {}),
            }}
          >
            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {f.name}
              {doc.startFrameId === f.id && <span style={badgeStyle}>開始</span>}
            </span>
            {doc.startFrameId !== f.id && (
              <button
                style={tinyBtnStyle}
                title="開始フレームにする"
                onClick={(e) => { e.stopPropagation(); setStartFrame(f.id); }}
              >
                ▶開始
              </button>
            )}
            <button
              style={tinyBtnStyle}
              title="フレームを削除"
              disabled={doc.frames.length <= 1}
              onClick={(e) => { e.stopPropagation(); deleteFrame(f.id); }}
            >
              🗑
            </button>
          </div>
        ))}
      </div>

      <div style={sectionStyle}>
        <div style={sectionHeaderStyle}>
          <span>レイヤー（上が手前）</span>
        </div>
        {[...frame.nodes].reverse().map((n) => (
          <div
            key={n.id}
            onClick={() => selectNode(n.id)}
            style={{
              ...rowStyle,
              ...(n.id === selectedNodeId ? rowActiveStyle : {}),
              opacity: n.visible === false ? 0.45 : 1,
            }}
          >
            <span style={typeBadgeStyle}>{n.type}</span>
            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {n.name}
            </span>
            <button
              style={tinyBtnStyle}
              title="前面へ"
              onClick={(e) => { e.stopPropagation(); commit(moveNodeLayer(doc, frame.id, n.id, 1)); }}
            >
              ↑
            </button>
            <button
              style={tinyBtnStyle}
              title="背面へ"
              onClick={(e) => { e.stopPropagation(); commit(moveNodeLayer(doc, frame.id, n.id, -1)); }}
            >
              ↓
            </button>
            <button
              style={tinyBtnStyle}
              title="表示切替"
              onClick={(e) => {
                e.stopPropagation();
                patchNode(n.id, { visible: n.visible === false ? true : false } as never);
              }}
            >
              {n.visible === false ? '🙈' : '👁'}
            </button>
            <button
              style={tinyBtnStyle}
              title="削除"
              onClick={(e) => {
                e.stopPropagation();
                commit(removeNodeFromFrame(doc, frame.id, n.id));
                if (selectedNodeId === n.id) selectNode(null);
              }}
            >
              🗑
            </button>
          </div>
        ))}
        {frame.nodes.length === 0 && <div style={{ color: '#8496b3', fontSize: 12, padding: 6 }}>ノードがありません</div>}
      </div>
    </div>
  );
}

const panelStyle: React.CSSProperties = {
  width: 232, background: '#1d2534', color: '#dbe6f5', overflowY: 'auto',
  borderRight: '1px solid #11161f', fontSize: 13, flexShrink: 0,
};
const sectionStyle: React.CSSProperties = { padding: '10px 10px 14px', borderBottom: '1px solid #11161f' };
const sectionHeaderStyle: React.CSSProperties = {
  display: 'flex', justifyContent: 'space-between', alignItems: 'center',
  fontWeight: 700, marginBottom: 8, color: '#eef3fa',
};
const rowStyle: React.CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 4, padding: '5px 6px',
  borderRadius: 6, cursor: 'pointer', marginBottom: 2,
};
const rowActiveStyle: React.CSSProperties = { background: '#2f4a7a' };
const smallBtnStyle: React.CSSProperties = {
  background: '#2a3850', color: '#dbe6f5', border: '1px solid #3a4f7a',
  borderRadius: 5, padding: '2px 8px', cursor: 'pointer', fontSize: 12,
};
const tinyBtnStyle: React.CSSProperties = {
  background: 'transparent', color: '#c7d3e6', border: 'none',
  cursor: 'pointer', fontSize: 11, padding: '2px 3px',
};
const badgeStyle: React.CSSProperties = {
  marginLeft: 6, background: '#2f6fd0', color: '#fff', borderRadius: 4,
  padding: '1px 6px', fontSize: 10,
};
const typeBadgeStyle: React.CSSProperties = {
  background: '#11161f', color: '#8ea6c8', borderRadius: 4,
  padding: '1px 5px', fontSize: 10, minWidth: 44, textAlign: 'center',
};
