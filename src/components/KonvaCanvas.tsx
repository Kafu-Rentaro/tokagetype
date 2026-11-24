import { useEffect, useRef, useState } from 'react';
import { Group, Layer, Rect, Stage, Text, Transformer } from 'react-konva';
import type Konva from 'konva';
import type { ButtonNode, Node, RectNode, TextNode, View3DNode } from '../model/types';
import { createNode } from '../model/factory';
import { addNodeToFrame } from '../model/mutate';
import { useEditorStore } from '../store/editorStore';
import { useUiStore } from '../store/uiStore';
import View3DCanvas from '../three/View3DCanvas';
import { addAssetFromFile, assignAssetToNode } from '../lib/assets';
import { miniBtn, miniBtnActive } from '../ui/theme';

function View3DOverlay({ node, scale }: { node: View3DNode; scale: number }) {
  const orbitNodeId = useEditorStore((s) => s.orbitNodeId);
  const camera = node.cameras.find((c) => c.id === node.activeCameraId) ?? node.cameras[0];
  if (!camera) return null;
  const orbiting = orbitNodeId === node.id;
  return (
    <div
      style={{
        position: 'absolute',
        left: node.x * scale,
        top: node.y * scale,
        width: node.width * scale,
        height: node.height * scale,
        pointerEvents: orbiting ? 'auto' : 'none',
        outline: orbiting ? '2px solid var(--accent)' : undefined,
        background: '#dde1e6',
        overflow: 'hidden',
      }}
    >
      <View3DCanvas
        nodeId={node.id}
        assetId={node.assetId}
        camera={camera}
        interactive={orbiting}
        autoRotate={node.autoRotate ?? false}
      />
    </div>
  );
}

const clampScale = (v: number) => Math.min(3, Math.max(0.15, v));

export default function KonvaCanvas() {
  const doc = useEditorStore((s) => s.doc);
  const frame = useEditorStore((s) => s.currentFrame());
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);
  const selectNode = useEditorStore((s) => s.selectNode);
  const patchNode = useEditorStore((s) => s.patchNode);
  const commit = useEditorStore((s) => s.commit);
  const uiMode = useUiStore((s) => s.uiMode);
  const mobile = uiMode === 'mobile';

  const [scale, setScale] = useState(0.85);
  const [editingTextId, setEditingTextId] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const transformerRef = useRef<Konva.Transformer>(null);
  const nodeRefs = useRef(new Map<string, Konva.Node>());
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const pinchRef = useRef<{ dist: number; scale: number } | null>(null);

  useEffect(() => {
    const tr = transformerRef.current;
    if (!tr) return;
    const target = selectedNodeId ? nodeRefs.current.get(selectedNodeId) : undefined;
    tr.nodes(target ? [target] : []);
    tr.getLayer()?.batchDraw();
  }, [selectedNodeId, frame?.nodes]);

  // スマホ UI: フレーム幅が画面に収まるよう初期ズームをフィットさせる
  useEffect(() => {
    if (!mobile || !frame) return;
    const el = scrollRef.current;
    if (!el) return;
    const fit = () => {
      const w = el.clientWidth - 24;
      if (w > 0) setScale(clampScale(Math.min(1, w / frame.width)));
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
    // フレーム切替時のみ再フィット（手動ズームを毎回潰さない）
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mobile, frame?.id]);

  // ピンチズーム（2本指）。Konva のタッチドラッグと競合しないよう、
  // 2本指のときだけ介入し、スクロールを止める。
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const dist = (t: TouchList) =>
      Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY);
    const onStart = (e: TouchEvent) => {
      if (e.touches.length === 2) {
        pinchRef.current = { dist: dist(e.touches), scale };
      }
    };
    const onMove = (e: TouchEvent) => {
      if (e.touches.length === 2 && pinchRef.current) {
        e.preventDefault();
        const d = dist(e.touches);
        if (pinchRef.current.dist > 0) {
          setScale(clampScale(pinchRef.current.scale * (d / pinchRef.current.dist)));
        }
      }
    };
    const onEnd = (e: TouchEvent) => {
      if (e.touches.length < 2) pinchRef.current = null;
    };
    el.addEventListener('touchstart', onStart, { passive: true });
    el.addEventListener('touchmove', onMove, { passive: false });
    el.addEventListener('touchend', onEnd);
    el.addEventListener('touchcancel', onEnd);
    return () => {
      el.removeEventListener('touchstart', onStart);
      el.removeEventListener('touchmove', onMove);
      el.removeEventListener('touchend', onEnd);
      el.removeEventListener('touchcancel', onEnd);
    };
  }, [scale]);

  if (!doc || !frame) return <div style={{ padding: 24 }}>読み込み中…</div>;

  const handleSelect = (node: Node) => selectNode(node.id);

  const commonProps = (node: Node) => ({
    id: node.id,
    x: node.x,
    y: node.y,
    rotation: node.rotation ?? 0,
    opacity: node.opacity ?? 1,
    visible: node.visible !== false,
    draggable: true,
    onClick: () => handleSelect(node),
    onTap: () => handleSelect(node),
    onDragEnd: (e: Konva.KonvaEventObject<DragEvent>) => {
      patchNode(node.id, { x: Math.round(e.target.x()), y: Math.round(e.target.y()) } as Partial<Node>);
    },
    onTransformEnd: (e: Konva.KonvaEventObject<Event>) => {
      const target = e.target;
      const scaleX = target.scaleX();
      const scaleY = target.scaleY();
      const newWidth = Math.max(10, Math.round(node.width * scaleX));
      const newHeight = Math.max(10, Math.round(node.height * scaleY));
      target.scaleX(1);
      target.scaleY(1);
      patchNode(node.id, {
        x: Math.round(target.x()),
        y: Math.round(target.y()),
        width: newWidth,
        height: newHeight,
        rotation: Math.round(target.rotation()),
      } as Partial<Node>);
    },
    ref: (instance: Konva.Node | null) => {
      if (instance) nodeRefs.current.set(node.id, instance);
      else nodeRefs.current.delete(node.id);
    },
  });

  const renderNode = (node: Node) => {
    switch (node.type) {
      case 'rect': {
        const n = node as RectNode;
        return (
          <Rect
            key={n.id}
            {...commonProps(n)}
            width={n.width}
            height={n.height}
            fill={n.fill}
            cornerRadius={n.cornerRadius}
            stroke={n.stroke}
            strokeWidth={n.strokeWidth}
          />
        );
      }
      case 'text': {
        const n = node as TextNode;
        return (
          <Text
            key={n.id}
            {...commonProps(n)}
            width={n.width}
            height={n.height}
            text={n.text}
            fontSize={n.fontSize}
            fill={n.fill}
            fontStyle={n.fontStyle === 'bold' ? 'bold' : 'normal'}
            align={n.align ?? 'left'}
            onDblClick={() => setEditingTextId(n.id)}
            onDblTap={() => setEditingTextId(n.id)}
          />
        );
      }
      case 'button': {
        const n = node as ButtonNode;
        return (
          <Group key={n.id} {...commonProps(n)}>
            <Rect
              width={n.width}
              height={n.height}
              fill={n.fill}
              cornerRadius={n.cornerRadius}
            />
            <Text
              width={n.width}
              height={n.height}
              text={n.label}
              fontSize={n.fontSize}
              fill={n.textColor}
              align="center"
              verticalAlign="middle"
            />
          </Group>
        );
      }
      case 'view3d': {
        const n = node as View3DNode;
        return (
          <Group key={n.id} {...commonProps(n)}>
            <Rect
              width={n.width}
              height={n.height}
              fill="#dde1e6"
              stroke="#9b9ca4"
              strokeWidth={1}
              dash={[6, 4]}
            />
            <Text x={8} y={8} text="3D" fontSize={12} fill="#5a5e66" />
          </Group>
        );
      }
    }
  };

  const editingTextNode = editingTextId
    ? (frame.nodes.find((n) => n.id === editingTextId && n.type === 'text') as TextNode | undefined)
    : undefined;

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const file = [...e.dataTransfer.files].find(
      (f) => f.name.toLowerCase().endsWith('.glb') || f.type === 'model/gltf-binary',
    );
    if (!file) return;
    const rect = containerRef.current?.getBoundingClientRect();
    const px = rect ? (e.clientX - rect.left) / scale : 0;
    const py = rect ? (e.clientY - rect.top) / scale : 0;
    const assetId = await addAssetFromFile(file);
    if (!assetId) return;
    // ドロップ位置にある view3d があれば割り当て、無ければ新規 view3d を作って割り当てる
    const hit = [...frame.nodes].reverse().find(
      (n) => n.type === 'view3d' && px >= n.x && px <= n.x + n.width && py >= n.y && py <= n.y + n.height,
    ) as View3DNode | undefined;
    if (hit) {
      assignAssetToNode(frame.id, hit.id, assetId);
      selectNode(hit.id);
    } else {
      const node = createNode('view3d', Math.max(0, Math.round(px - 155)), Math.max(0, Math.round(py - 190))) as View3DNode;
      node.assetId = assetId;
      node.name = file.name;
      commit(addNodeToFrame(useEditorStore.getState().doc!, frame.id, node));
      selectNode(node.id);
    }
  };

  const fitToWidth = () => {
    const el = scrollRef.current;
    if (!el) return;
    const w = el.clientWidth - (mobile ? 24 : 48);
    if (w > 0) setScale(clampScale(w / frame.width));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={zoomBarStyle}>
        <span style={{ whiteSpace: 'nowrap', flexShrink: 0 }}>ズーム</span>
        {[0.5, 0.75, 0.85, 1, 1.25, 1.5].map((z) => (
          <button
            key={z}
            onClick={() => setScale(z)}
            style={{ ...miniBtn, ...(scale === z ? miniBtnActive : {}) }}
          >
            <span className="num">{Math.round(z * 100)}%</span>
          </button>
        ))}
        <button onClick={fitToWidth} style={miniBtn}>幅に合わせる</button>
        <span style={{ marginLeft: 12, color: 'var(--text-dim)', whiteSpace: 'nowrap' }}>
          {frame.name}（<span className="num">{frame.width}×{frame.height}</span>）
        </span>
        <span className="num" style={{ marginLeft: 'auto', color: 'var(--text-faint)' }}>
          {Math.round(scale * 100)}%
        </span>
      </div>
      <div
        ref={scrollRef}
        className="dotgrid-bg"
        style={{ flex: 1, overflow: 'auto', padding: mobile ? 12 : 24 }}
        onClick={(e) => {
          if (e.target === e.currentTarget) selectNode(null);
        }}
      >
        <div
          ref={containerRef}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragLeave={() => setDragOver(false)}
          onDrop={handleDrop}
          style={{
            position: 'relative',
            width: frame.width * scale,
            height: frame.height * scale,
            margin: '0 auto',
            border: '1px solid var(--line-strong)',
            outline: dragOver ? '2px dashed var(--accent)' : undefined,
            background: frame.background,
          }}
        >
          <Stage
            width={frame.width * scale}
            height={frame.height * scale}
            onClick={(e) => {
              if (e.target === e.target.getStage()) selectNode(null);
            }}
          >
            <Layer scaleX={scale} scaleY={scale}>
              {frame.nodes.map(renderNode)}
              <Transformer
                ref={transformerRef}
                rotateEnabled
                keepRatio={false}
                anchorStroke="#ff4d1c"
                borderStroke="#ff4d1c"
                boundBoxFunc={(oldBox, newBox) => (newBox.width < 10 || newBox.height < 10 ? oldBox : newBox)}
              />
            </Layer>
          </Stage>

          {frame.nodes
            .filter((n): n is View3DNode => n.type === 'view3d' && n.visible !== false)
            .map((n) => (
              <View3DOverlay key={n.id} node={n} scale={scale} />
            ))}

          {editingTextNode && (
            <textarea
              autoFocus
              defaultValue={editingTextNode.text}
              onBlur={(e) => {
                patchNode(editingTextNode.id, { text: e.target.value } as Partial<Node>);
                setEditingTextId(null);
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  (e.target as HTMLTextAreaElement).blur();
                }
                if (e.key === 'Escape') setEditingTextId(null);
                e.stopPropagation();
              }}
              style={{
                position: 'absolute',
                left: editingTextNode.x * scale,
                top: editingTextNode.y * scale,
                width: editingTextNode.width * scale,
                height: Math.max(40, editingTextNode.height * scale),
                fontSize: editingTextNode.fontSize * scale,
                color: editingTextNode.fill,
                background: 'rgba(255,255,255,0.95)',
                border: '2px solid var(--accent)',
                borderRadius: 4,
                zIndex: 20,
                resize: 'both',
                fontFamily: 'inherit',
                textAlign: editingTextNode.align ?? 'left',
              }}
            />
          )}
        </div>
        {!mobile && (
          <p style={{ color: 'var(--text-faint)', fontSize: 12, textAlign: 'center', marginTop: 12 }}>
            GLB ファイルをキャンバスへドラッグ&ドロップすると 3D ビューに割り当てられます
          </p>
        )}
      </div>
    </div>
  );
}

const zoomBarStyle: React.CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 6, padding: '6px 12px',
  background: 'var(--panel)', color: 'var(--text-dim)', fontSize: 12,
  borderBottom: '1px solid var(--line)', overflowX: 'auto',
};
