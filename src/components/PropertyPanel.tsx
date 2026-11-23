import { useRef } from 'react';
import type {
  Action,
  ButtonNode,
  CameraDef,
  Node,
  RectNode,
  TextNode,
  View3DNode,
} from '../model/types';
import { defaultAction, newId } from '../model/factory';
import { cloneDoc } from '../model/mutate';
import { findFrame, validateAction } from '../model/validate';
import { useEditorStore } from '../store/editorStore';
import { getCameraSnapshot } from '../three/cameraRegistry';
import { addAssetFromFile, downloadAssetGLB } from '../lib/assets';

function NumField({ label, value, onChange, step = 1 }: {
  label: string; value: number; onChange: (v: number) => void; step?: number;
}) {
  return (
    <label style={fieldStyle}>
      <span style={fieldLabelStyle}>{label}</span>
      <input
        type="number"
        step={step}
        value={Math.round(value * 100) / 100}
        onChange={(e) => onChange(Number(e.target.value))}
        style={inputStyle}
      />
    </label>
  );
}

function ColorField({ label, value, onChange }: {
  label: string; value: string; onChange: (v: string) => void;
}) {
  return (
    <label style={fieldStyle}>
      <span style={fieldLabelStyle}>{label}</span>
      <span style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
        <input type="color" value={value} onChange={(e) => onChange(e.target.value)} style={{ width: 34, height: 26, padding: 0, border: 'none', background: 'none' }} />
        <input type="text" value={value} onChange={(e) => onChange(e.target.value)} style={{ ...inputStyle, flex: 1 }} />
      </span>
    </label>
  );
}

function ActionsEditor({ node }: { node: Node }) {
  const doc = useEditorStore((s) => s.doc);
  const frame = useEditorStore((s) => s.currentFrame());
  const patchNode = useEditorStore((s) => s.patchNode);
  if (!doc || !frame) return null;

  const actions = node.actions ?? [];
  const viewNodes = doc.frames.flatMap((f) => f.nodes).filter((n): n is View3DNode => n.type === 'view3d');
  const allNodes = doc.frames.flatMap((f) => f.nodes.map((n) => ({ frame: f, node: n })));

  const setActions = (next: Action[]) => patchNode(node.id, { actions: next } as Partial<Node>);
  const updateAction = (idx: number, next: Action) => {
    const copy = [...actions];
    copy[idx] = next;
    setActions(copy);
  };

  return (
    <div style={groupStyle}>
      <div style={groupTitleStyle}>クリック時の操作（Actions）</div>
      {actions.map((action, idx) => {
        const issues = validateAction(doc, action);
        return (
          <div key={idx} style={actionRowStyle}>
            <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
              <strong style={{ fontSize: 12 }}>{action.type}</strong>
              <button
                style={miniBtnStyle}
                onClick={() => setActions(actions.filter((_, i) => i !== idx))}
              >
                削除
              </button>
            </div>
            {action.type === 'setCamera' && (
              <>
                <label style={fieldStyle}>
                  <span style={fieldLabelStyle}>3Dビュー</span>
                  <select
                    value={action.viewNodeId}
                    onChange={(e) => {
                      const view = viewNodes.find((v) => v.id === e.target.value);
                      updateAction(idx, { ...action, viewNodeId: e.target.value, cameraId: view?.cameras[0]?.id ?? '' });
                    }}
                    style={inputStyle}
                  >
                    <option value="">（未選択）</option>
                    {viewNodes.map((v) => <option key={v.id} value={v.id}>{v.name}</option>)}
                  </select>
                </label>
                <label style={fieldStyle}>
                  <span style={fieldLabelStyle}>カメラ</span>
                  <select
                    value={action.cameraId}
                    onChange={(e) => updateAction(idx, { ...action, cameraId: e.target.value })}
                    style={inputStyle}
                  >
                    <option value="">（未選択）</option>
                    {(viewNodes.find((v) => v.id === action.viewNodeId)?.cameras ?? []).map((c) => (
                      <option key={c.id} value={c.id}>{c.name}</option>
                    ))}
                  </select>
                </label>
              </>
            )}
            {action.type === 'navigate' && (
              <label style={fieldStyle}>
                <span style={fieldLabelStyle}>遷移先フレーム</span>
                <select
                  value={action.frameId}
                  onChange={(e) => updateAction(idx, { ...action, frameId: e.target.value })}
                  style={inputStyle}
                >
                  <option value="">（未選択）</option>
                  {doc.frames.map((f) => <option key={f.id} value={f.id}>{f.name}</option>)}
                </select>
              </label>
            )}
            {action.type === 'toggleVisible' && (
              <label style={fieldStyle}>
                <span style={fieldLabelStyle}>対象ノード</span>
                <select
                  value={action.nodeId}
                  onChange={(e) => updateAction(idx, { ...action, nodeId: e.target.value })}
                  style={inputStyle}
                >
                  <option value="">（未選択）</option>
                  {allNodes.map(({ frame: f, node: n }) => (
                    <option key={n.id} value={n.id}>{f.name} / {n.name}</option>
                  ))}
                </select>
              </label>
            )}
            {action.type === 'openUrl' && (
              <label style={fieldStyle}>
                <span style={fieldLabelStyle}>URL</span>
                <input
                  type="text"
                  value={action.url}
                  onChange={(e) => updateAction(idx, { ...action, url: e.target.value })}
                  style={inputStyle}
                />
              </label>
            )}
            {issues.map((issue, i) => (
              <div key={i} style={{ color: '#ffb35c', fontSize: 11 }}>⚠ {issue.message}</div>
            ))}
          </div>
        );
      })}
      <div style={{ display: 'flex', gap: 6, marginTop: 6 }}>
        {(['setCamera', 'navigate', 'toggleVisible', 'openUrl'] as Action['type'][]).map((t) => (
          <button
            key={t}
            style={miniBtnStyle}
            onClick={() => setActions([...actions, defaultAction(t, doc, frame)])}
          >
            ＋{t}
          </button>
        ))}
      </div>
    </div>
  );
}

function CameraEditor({ node }: { node: View3DNode }) {
  const patchNode = useEditorStore((s) => s.patchNode);
  const orbitNodeId = useEditorStore((s) => s.orbitNodeId);
  const setOrbitNode = useEditorStore((s) => s.setOrbitNode);
  const orbiting = orbitNodeId === node.id;

  const setCameras = (cameras: CameraDef[], activeId?: string) =>
    patchNode(node.id, { cameras, ...(activeId ? { activeCameraId: activeId } : {}) } as Partial<Node>);

  const saveCurrentView = (overwriteId?: string) => {
    const snap = getCameraSnapshot(node.id);
    if (!snap) {
      alert('現在の視点を取得できませんでした。3Dビューが表示されている状態で「3D操作」を有効にしてからお試しください。');
      return;
    }
    if (overwriteId) {
      setCameras(node.cameras.map((c) => (c.id === overwriteId ? { ...c, ...snap } : c)));
    } else {
      const cam: CameraDef = { id: newId('cam'), name: `camera ${node.cameras.length + 1}`, ...snap };
      setCameras([...node.cameras, cam], cam.id);
    }
  };

  return (
    <div style={groupStyle}>
      <div style={groupTitleStyle}>カメラ</div>
      <label style={{ display: 'flex', gap: 6, alignItems: 'center', fontSize: 12, marginBottom: 8 }}>
        <input
          type="checkbox"
          checked={orbiting}
          onChange={(e) => setOrbitNode(e.target.checked ? node.id : null)}
        />
        3D操作（OrbitControls）を有効化
      </label>
      {node.cameras.map((cam) => (
        <div key={cam.id} style={{ display: 'flex', gap: 4, alignItems: 'center', marginBottom: 4 }}>
          <input
            type="text"
            value={cam.name}
            onChange={(e) => setCameras(node.cameras.map((c) => (c.id === cam.id ? { ...c, name: e.target.value } : c)))}
            style={{ ...inputStyle, flex: 1 }}
          />
          <button
            style={{ ...miniBtnStyle, ...(node.activeCameraId === cam.id ? miniBtnActiveStyle : {}) }}
            onClick={() => patchNode(node.id, { activeCameraId: cam.id } as Partial<Node>)}
            title="このカメラを表示"
          >
            表示
          </button>
          <button
            style={miniBtnStyle}
            onClick={() => saveCurrentView(cam.id)}
            title="現在の視点で上書き"
          >
            視点で上書
          </button>
          <button
            style={miniBtnStyle}
            disabled={node.cameras.length <= 1}
            onClick={() => {
              const rest = node.cameras.filter((c) => c.id !== cam.id);
              setCameras(rest, node.activeCameraId === cam.id ? rest[0]?.id : undefined);
            }}
          >
            ✕
          </button>
        </div>
      ))}
      <button style={{ ...miniBtnStyle, marginTop: 4 }} onClick={() => saveCurrentView()}>
        ＋ 現在の視点をカメラとして保存
      </button>
      <label style={{ display: 'flex', gap: 6, alignItems: 'center', fontSize: 12, marginTop: 8 }}>
        <input
          type="checkbox"
          checked={node.autoRotate ?? false}
          onChange={(e) => patchNode(node.id, { autoRotate: e.target.checked } as Partial<Node>)}
        />
        自動回転
      </label>
    </div>
  );
}

export default function PropertyPanel() {
  const doc = useEditorStore((s) => s.doc);
  const frame = useEditorStore((s) => s.currentFrame());
  const node = useEditorStore((s) => s.selectedNode());
  const patchNode = useEditorStore((s) => s.patchNode);
  const patchFrame = useEditorStore((s) => s.patchFrame);
  const commit = useEditorStore((s) => s.commit);
  const setSculptNode = useEditorStore((s) => s.setSculptNode);
  const fileRef = useRef<HTMLInputElement>(null);

  if (!doc || !frame) return <div style={panelStyle} />;

  if (!node) {
    const targetFrame = findFrame(doc, frame.id) ?? frame;
    return (
      <div style={panelStyle}>
        <div style={groupStyle}>
          <div style={groupTitleStyle}>プロジェクト</div>
          <label style={fieldStyle}>
            <span style={fieldLabelStyle}>名前</span>
            <input
              type="text"
              value={doc.name}
              onChange={(e) => {
                const next = cloneDoc(doc);
                next.name = e.target.value;
                commit(next);
              }}
              style={inputStyle}
            />
          </label>
        </div>
        <div style={groupStyle}>
          <div style={groupTitleStyle}>フレーム: {targetFrame.name}</div>
          <label style={fieldStyle}>
            <span style={fieldLabelStyle}>名前</span>
            <input
              type="text"
              value={targetFrame.name}
              onChange={(e) => patchFrame(targetFrame.id, { name: e.target.value })}
              style={inputStyle}
            />
          </label>
          <NumField label="幅" value={targetFrame.width} onChange={(v) => patchFrame(targetFrame.id, { width: v })} />
          <NumField label="高さ" value={targetFrame.height} onChange={(v) => patchFrame(targetFrame.id, { height: v })} />
          <ColorField label="背景色" value={targetFrame.background} onChange={(v) => patchFrame(targetFrame.id, { background: v })} />
          <p style={{ color: '#8496b3', fontSize: 12 }}>
            ノードを選択すると、ここでプロパティ・カメラ・操作（Action）を編集できます。
          </p>
        </div>
      </div>
    );
  }

  const patch = (p: Record<string, unknown>) => patchNode(node.id, p as Partial<Node>);

  return (
    <div style={panelStyle}>
      <div style={groupStyle}>
        <div style={groupTitleStyle}>{node.type}: 名前・配置</div>
        <label style={fieldStyle}>
          <span style={fieldLabelStyle}>名前</span>
          <input type="text" value={node.name} onChange={(e) => patch({ name: e.target.value })} style={inputStyle} />
        </label>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0 8px' }}>
          <NumField label="X" value={node.x} onChange={(v) => patch({ x: v })} />
          <NumField label="Y" value={node.y} onChange={(v) => patch({ y: v })} />
          <NumField label="幅" value={node.width} onChange={(v) => patch({ width: v })} />
          <NumField label="高さ" value={node.height} onChange={(v) => patch({ height: v })} />
          <NumField label="回転°" value={node.rotation ?? 0} onChange={(v) => patch({ rotation: v })} />
          <NumField label="不透明度" value={node.opacity ?? 1} step={0.05} onChange={(v) => patch({ opacity: Math.min(1, Math.max(0, v)) })} />
        </div>
        <label style={{ display: 'flex', gap: 6, alignItems: 'center', fontSize: 12 }}>
          <input
            type="checkbox"
            checked={node.visible !== false}
            onChange={(e) => patch({ visible: e.target.checked })}
          />
          表示する
        </label>
      </div>

      {node.type === 'rect' && (() => {
        const n = node as RectNode;
        return (
          <div style={groupStyle}>
            <div style={groupTitleStyle}>矩形</div>
            <ColorField label="塗り" value={n.fill} onChange={(v) => patch({ fill: v })} />
            <NumField label="角丸" value={n.cornerRadius} onChange={(v) => patch({ cornerRadius: v })} />
            <ColorField label="枠線" value={n.stroke ?? '#000000'} onChange={(v) => patch({ stroke: v })} />
            <NumField label="枠線幅" value={n.strokeWidth ?? 0} onChange={(v) => patch({ strokeWidth: v })} />
          </div>
        );
      })()}

      {node.type === 'text' && (() => {
        const n = node as TextNode;
        return (
          <div style={groupStyle}>
            <div style={groupTitleStyle}>テキスト（ダブルクリックでも編集可）</div>
            <label style={fieldStyle}>
              <span style={fieldLabelStyle}>内容</span>
              <textarea value={n.text} rows={3} onChange={(e) => patch({ text: e.target.value })} style={inputStyle} />
            </label>
            <NumField label="フォントサイズ" value={n.fontSize} onChange={(v) => patch({ fontSize: v })} />
            <ColorField label="色" value={n.fill} onChange={(v) => patch({ fill: v })} />
            <label style={fieldStyle}>
              <span style={fieldLabelStyle}>揃え</span>
              <select value={n.align ?? 'left'} onChange={(e) => patch({ align: e.target.value })} style={inputStyle}>
                <option value="left">左</option>
                <option value="center">中央</option>
                <option value="right">右</option>
              </select>
            </label>
            <label style={{ display: 'flex', gap: 6, alignItems: 'center', fontSize: 12 }}>
              <input
                type="checkbox"
                checked={n.fontStyle === 'bold'}
                onChange={(e) => patch({ fontStyle: e.target.checked ? 'bold' : 'normal' })}
              />
              太字
            </label>
          </div>
        );
      })()}

      {node.type === 'button' && (() => {
        const n = node as ButtonNode;
        return (
          <div style={groupStyle}>
            <div style={groupTitleStyle}>ボタン</div>
            <label style={fieldStyle}>
              <span style={fieldLabelStyle}>ラベル</span>
              <input type="text" value={n.label} onChange={(e) => patch({ label: e.target.value })} style={inputStyle} />
            </label>
            <ColorField label="背景" value={n.fill} onChange={(v) => patch({ fill: v })} />
            <ColorField label="文字色" value={n.textColor} onChange={(v) => patch({ textColor: v })} />
            <NumField label="角丸" value={n.cornerRadius} onChange={(v) => patch({ cornerRadius: v })} />
            <NumField label="フォントサイズ" value={n.fontSize} onChange={(v) => patch({ fontSize: v })} />
          </div>
        );
      })()}

      {node.type === 'view3d' && (() => {
        const n = node as View3DNode;
        return (
          <>
            <div style={groupStyle}>
              <div style={groupTitleStyle}>3Dモデル（GLB）</div>
              <label style={fieldStyle}>
                <span style={fieldLabelStyle}>アセット</span>
                <select
                  value={n.assetId ?? ''}
                  onChange={(e) => patch({ assetId: e.target.value || null })}
                  style={inputStyle}
                >
                  <option value="">（なし）</option>
                  {doc.assets.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
                </select>
              </label>
              <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                <button style={miniBtnStyle} onClick={() => fileRef.current?.click()}>
                  GLB ファイルを選択…
                </button>
                <input
                  ref={fileRef}
                  type="file"
                  accept=".glb,model/gltf-binary"
                  style={{ display: 'none' }}
                  onChange={async (e) => {
                    const file = e.target.files?.[0];
                    e.target.value = '';
                    if (!file) return;
                    const assetId = await addAssetFromFile(file);
                    if (assetId) patch({ assetId });
                  }}
                />
                <button
                  style={miniBtnStyle}
                  disabled={!n.assetId}
                  onClick={() => { if (n.assetId) void downloadAssetGLB(n.assetId, doc.assets.find((a) => a.id === n.assetId)?.name); }}
                >
                  GLB 書き出し
                </button>
                <button style={{ ...miniBtnStyle, background: '#2f6fd0', color: '#fff' }} onClick={() => setSculptNode(n.id)}>
                  🗿 スカルプト
                </button>
              </div>
            </div>
            <CameraEditor node={n} />
          </>
        );
      })()}

      <ActionsEditor node={node} />
    </div>
  );
}

const panelStyle: React.CSSProperties = {
  width: 288, background: '#1d2534', color: '#dbe6f5', overflowY: 'auto',
  borderLeft: '1px solid #11161f', fontSize: 13, flexShrink: 0,
};
const groupStyle: React.CSSProperties = { padding: '10px 12px 14px', borderBottom: '1px solid #11161f' };
const groupTitleStyle: React.CSSProperties = { fontWeight: 700, color: '#eef3fa', marginBottom: 8, fontSize: 12 };
const fieldStyle: React.CSSProperties = { display: 'block', marginBottom: 8 };
const fieldLabelStyle: React.CSSProperties = { display: 'block', fontSize: 11, color: '#8ea6c8', marginBottom: 3 };
const inputStyle: React.CSSProperties = {
  width: '100%', background: '#11161f', color: '#e6eefb', border: '1px solid #3a4f7a',
  borderRadius: 5, padding: '5px 7px', fontSize: 13, boxSizing: 'border-box',
};
const miniBtnStyle: React.CSSProperties = {
  background: '#2a3850', color: '#dbe6f5', border: '1px solid #3a4f7a',
  borderRadius: 5, padding: '3px 8px', cursor: 'pointer', fontSize: 11,
};
const miniBtnActiveStyle: React.CSSProperties = { background: '#2f6fd0', color: '#fff' };
const actionRowStyle: React.CSSProperties = {
  background: '#16202f', border: '1px solid #2a3850', borderRadius: 6,
  padding: 8, marginBottom: 8,
};
