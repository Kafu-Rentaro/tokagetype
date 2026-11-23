import { useEffect, useRef, useState } from 'react';
import { Group, Layer, Rect, Stage, Text, Transformer } from 'react-konva';
import type Konva from 'konva';
import type { ButtonNode, Node, RectNode, TextNode, View3DNode } from '../model/types';
import { createNode } from '../model/factory';
import { addNodeToFrame } from '../model/mutate';
import { useEditorStore } from '../store/editorStore';
import { useAssetUrl } from '../three/assetUrls';
import View3DCanvas from '../three/View3DCanvas';
import { addAssetFromFile, assignAssetToNode } from '../lib/assets';

function View3DOverlay({ node, scale }: { node: View3DNode; scale: number }) {
  const orbitNodeId = useEditorStore((s) => s.orbitNodeId);
  const assetUrl = useAssetUrl(node.assetId);
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
        outline: orbiting ? '2px solid #2f6fd0' : undefined,
        background: '#e8eef7',
        overflow: 'hidden',
      }}
    >
      <View3DCanvas
        nodeId={node.id}
        assetUrl={assetUrl}
        camera={camera}
        interactive={orbiting}
        autoRotate={node.autoRotate ?? false}
      />
      {!node.assetId && (
        <div style={{
          position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center',
          color: '#5a6c85', fontSize: 13, pointerEvents: 'none', textAlign: 'center', padding: 8,
        }}>
          GLB 未設定<br />（ファイルを選択 / ドロップ）
        </div>
      )}
    </div>
  );
}

export default function KonvaCanvas() {
  const doc = useEditorStore((s) => s.doc);
  const frame = useEditorStore((s) => s.currentFrame());
  const selectedNodeId = useEditorStore((s) => s.selectedNodeId);
  const selectNode = useEditorStore((s) => s.selectNode);
  const patchNode = useEditorStore((s) => s.patchNode);
  const commit = useEditorStore((s) => s.commit);

  const [scale, setScale] = useState(0.85);
  const [editingTextId, setEditingTextId] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const transformerRef = useRef<Konva.Transformer>(null);
  const nodeRefs = useRef(new Map<string, Konva.Node>());
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const tr = transformerRef.current;
    if (!tr) return;
    const target = selectedNodeId ? nodeRefs.current.get(selectedNodeId) : undefined;
    tr.nodes(target ? [target] : []);
    tr.getLayer()?.batchDraw();
  }, [selectedNodeId, frame?.nodes]);

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
              fill="#dfe8f5"
              stroke="#8ea6c8"
              strokeWidth={1}
              dash={[6, 4]}
            />
            <Text x={8} y={8} text="3D" fontSize={12} fill="#5a6c85" />
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

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={zoomBarStyle}>
        <span>ズーム</span>
        {[0.5, 0.75, 0.85, 1, 1.25, 1.5].map((z) => (
          <button
            key={z}
            onClick={() => setScale(z)}
            style={{ ...zoomBtnStyle, ...(scale === z ? zoomBtnActiveStyle : {}) }}
          >
            {Math.round(z * 100)}%
          </button>
        ))}
        <span style={{ marginLeft: 12, color: '#5a6c85' }}>
          {frame.name}（{frame.width}×{frame.height}）
        </span>
      </div>
      <div
        style={{ flex: 1, overflow: 'auto', padding: 24, background: '#2a3140' }}
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
            boxShadow: '0 4px 24px rgba(0,0,0,0.45)',
            outline: dragOver ? '3px dashed #2f6fd0' : undefined,
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
                background: 'rgba(255,255,255,0.92)',
                border: '2px solid #2f6fd0',
                borderRadius: 4,
                zIndex: 20,
                resize: 'both',
                fontFamily: 'inherit',
                textAlign: editingTextNode.align ?? 'left',
              }}
            />
          )}
        </div>
        <p style={{ color: '#8ea0bb', fontSize: 12, textAlign: 'center', marginTop: 12 }}>
          GLB ファイルをキャンバスへドラッグ&ドロップすると 3D ビューに割り当てられます
        </p>
      </div>
    </div>
  );
}

const zoomBarStyle: React.CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 6, padding: '6px 12px',
  background: '#1d2534', color: '#c7d3e6', fontSize: 12,
};
const zoomBtnStyle: React.CSSProperties = {
  background: '#2a3850', color: '#c7d3e6', border: '1px solid #3a4f7a',
  borderRadius: 4, padding: '2px 8px', cursor: 'pointer', fontSize: 12,
};
const zoomBtnActiveStyle: React.CSSProperties = { background: '#2f6fd0', color: '#fff' };
