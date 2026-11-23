import { createFrame } from '../model/factory';
import { cloneDoc, moveNodeLayer, removeNodeFromFrame } from '../model/mutate';
import { useEditorStore } from '../store/editorStore';
import { IconChevronDown, IconChevronUp, IconEye, IconEyeOff, IconPlus, IconTrash } from '../ui/icons';
import { iconBtn, miniBtn, panelLeft } from '../ui/theme';

export default function Panels({ style }: { style?: React.CSSProperties }) {
  const doc = useEditorStore((s) => s.doc);
  const frame = useEditorStore((s) => s.currentFrame());
  const currentFrameId = useEditorStore((s) => s.currentFrameId);
  const setCurrentFrame = useEditorStore((s) => s.setCurrentFrame);
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);
  const selectNode = useEditorStore((s) => s.selectNode);
  const commit = useEditorStore((s) => s.commit);
  const patchNode = useEditorStore((s) => s.patchNode);

  if (!doc || !frame) return <div style={{ ...panelLeft, ...style }} />;

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
    <div style={{ ...panelLeft, ...style }}>
      <div style={sectionStyle}>
        <div style={sectionHeaderStyle}>
          <span>フレーム</span>
          <button style={miniBtn} onClick={addFrame}><IconPlus size={12} /> 追加</button>
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
                style={miniBtn}
                title="開始フレームにする"
                onClick={(e) => { e.stopPropagation(); setStartFrame(f.id); }}
              >
                開始にする
              </button>
            )}
            <button
              style={iconBtn}
              title="フレームを削除"
              disabled={doc.frames.length <= 1}
              onClick={(e) => { e.stopPropagation(); deleteFrame(f.id); }}
            >
              <IconTrash size={14} />
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
              style={iconBtn}
              title="前面へ"
              onClick={(e) => { e.stopPropagation(); commit(moveNodeLayer(doc, frame.id, n.id, 1)); }}
            >
              <IconChevronUp size={14} />
            </button>
            <button
              style={iconBtn}
              title="背面へ"
              onClick={(e) => { e.stopPropagation(); commit(moveNodeLayer(doc, frame.id, n.id, -1)); }}
            >
              <IconChevronDown size={14} />
            </button>
            <button
              style={iconBtn}
              title="表示切替"
              onClick={(e) => {
                e.stopPropagation();
                patchNode(n.id, { visible: n.visible === false ? true : false } as never);
              }}
            >
              {n.visible === false ? <IconEyeOff size={14} /> : <IconEye size={14} />}
            </button>
            <button
              style={iconBtn}
              title="削除"
              onClick={(e) => {
                e.stopPropagation();
                commit(removeNodeFromFrame(doc, frame.id, n.id));
                if (selectedNodeId === n.id) selectNode(null);
              }}
            >
              <IconTrash size={14} />
            </button>
          </div>
        ))}
        {frame.nodes.length === 0 && <div style={{ color: 'var(--text-faint)', fontSize: 12, padding: 6 }}>ノードがありません</div>}
      </div>
    </div>
  );
}

const sectionStyle: React.CSSProperties = { padding: '12px 10px 14px', borderBottom: '1px solid var(--line)' };
const sectionHeaderStyle: React.CSSProperties = {
  display: 'flex', justifyContent: 'space-between', alignItems: 'center',
  fontWeight: 700, marginBottom: 8, color: 'var(--text)', fontSize: 11, letterSpacing: '0.08em',
};
const rowStyle: React.CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 4, padding: '6px 6px',
  borderRadius: 'var(--radius)', cursor: 'pointer', marginBottom: 2, minHeight: 32,
};
const rowActiveStyle: React.CSSProperties = {
  background: 'var(--panel-2)', boxShadow: 'inset 2px 0 0 var(--accent)',
};
const badgeStyle: React.CSSProperties = {
  marginLeft: 6, background: 'var(--accent)', color: 'var(--accent-ink)', borderRadius: 3,
  padding: '1px 6px', fontSize: 10,
};
const typeBadgeStyle: React.CSSProperties = {
  background: 'var(--inset)', color: 'var(--text-dim)', borderRadius: 3,
  padding: '1px 5px', fontSize: 10, minWidth: 46, textAlign: 'center',
  fontFamily: 'var(--mono)', textTransform: 'uppercase',
};
