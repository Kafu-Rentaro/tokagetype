import { useEffect, useState } from 'react';
import type { ButtonNode, Node, RectNode, TextNode, View3DNode } from '../model/types';
import { useEditorStore } from '../store/editorStore';
import { usePlayStore } from '../store/playStore';
import { useUiStore } from '../store/uiStore';
import View3DCanvas from '../three/View3DCanvas';
import { IconBack, IconPlay } from '../ui/icons';
import { btn } from '../ui/theme';

function PlayView3D({ node }: { node: View3DNode }) {
  const override = usePlayStore((s) => s.cameraOverrides[node.id]);
  const nonce = usePlayStore((s) => s.cameraNonce[node.id] ?? 0);
  const cameraId = override ?? node.activeCameraId;
  const camera = node.cameras.find((c) => c.id === cameraId) ?? node.cameras[0];
  if (!camera) return null;
  return (
    <View3DCanvas
      nodeId={node.id}
      assetId={node.assetId}
      camera={camera}
      animate
      animKey={nonce}
      interactive={false}
      autoRotate={node.autoRotate ?? false}
    />
  );
}

function PlayNode({ node }: { node: Node }) {
  const doc = useEditorStore((s) => s.doc);
  const runActions = usePlayStore((s) => s.runActions);
  const overrideVisible = usePlayStore((s) => s.visibilityOverrides[node.id]);
  const visible = overrideVisible ?? node.visible !== false;
  if (!visible || !doc) return null;

  const hasActions = (node.actions?.length ?? 0) > 0;
  const base: React.CSSProperties = {
    position: 'absolute',
    left: node.x,
    top: node.y,
    width: node.width,
    height: node.height,
    transform: node.rotation ? `rotate(${node.rotation}deg)` : undefined,
    opacity: node.opacity ?? 1,
    cursor: hasActions ? 'pointer' : undefined,
  };
  const onClick = hasActions ? () => runActions(doc, node.actions) : undefined;

  switch (node.type) {
    case 'rect': {
      const n = node as RectNode;
      return (
        <div
          onClick={onClick}
          style={{
            ...base,
            background: n.fill,
            borderRadius: n.cornerRadius,
            border: n.stroke ? `${n.strokeWidth ?? 1}px solid ${n.stroke}` : undefined,
            boxSizing: 'border-box',
          }}
        />
      );
    }
    case 'text': {
      const n = node as TextNode;
      return (
        <div
          onClick={onClick}
          style={{
            ...base,
            fontSize: n.fontSize,
            color: n.fill,
            fontWeight: n.fontStyle === 'bold' ? 700 : 400,
            textAlign: n.align ?? 'left',
            whiteSpace: 'pre-wrap',
            lineHeight: 1.35,
            overflow: 'hidden',
          }}
        >
          {n.text}
        </div>
      );
    }
    case 'button': {
      const n = node as ButtonNode;
      return (
        <button
          onClick={() => runActions(doc, n.actions)}
          style={{
            ...base,
            background: n.fill,
            color: n.textColor,
            borderRadius: n.cornerRadius,
            fontSize: n.fontSize,
            border: 'none',
            cursor: 'pointer',
            fontFamily: 'inherit',
          }}
        >
          {n.label}
        </button>
      );
    }
    case 'view3d': {
      const n = node as View3DNode;
      return (
        <div onClick={onClick} style={{ ...base, background: '#dde1e6', overflow: 'hidden' }}>
          <PlayView3D node={n} />
        </div>
      );
    }
  }
}

export default function PlayView() {
  const doc = useEditorStore((s) => s.doc);
  const frameId = usePlayStore((s) => s.frameId);
  const start = usePlayStore((s) => s.start);
  const stop = usePlayStore((s) => s.stop);
  const backToStart = usePlayStore((s) => s.backToStart);
  const mobile = useUiStore((s) => s.uiMode) === 'mobile';
  const [viewportWidth, setViewportWidth] = useState(() =>
    typeof window !== 'undefined' ? window.innerWidth : 1280,
  );

  useEffect(() => {
    if (doc) start(doc);
    return () => stop();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [doc?.id]);

  useEffect(() => {
    const onResize = () => setViewportWidth(window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  if (!doc) return null;
  const frame = doc.frames.find((f) => f.id === frameId) ?? doc.frames.find((f) => f.id === doc.startFrameId) ?? doc.frames[0];
  if (!frame) return null;

  // スマホ UI: フレームを画面幅にフィットさせる
  const fit = mobile ? Math.min(1, Math.max(0.2, (viewportWidth - 24) / frame.width)) : 1;

  return (
    <div
      className="dotgrid-bg"
      style={{
        flex: 1, overflow: 'auto', display: 'flex',
        flexDirection: 'column', alignItems: 'center', padding: mobile ? 12 : 20, gap: 12,
      }}
    >
      <div style={{ display: 'flex', gap: 10, alignItems: 'center', color: 'var(--text)', fontSize: 13, flexWrap: 'wrap', justifyContent: 'center' }}>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
          <IconPlay size={13} /> 再生中: <strong>{frame.name}</strong>
        </span>
        {frame.id !== doc.startFrameId && (
          <button onClick={() => backToStart(doc)} style={btn}>
            <IconBack size={14} /> 開始フレームに戻る
          </button>
        )}
      </div>
      <div style={{ width: frame.width * fit, height: frame.height * fit, flexShrink: 0 }}>
        <div
          style={{
            position: 'relative',
            width: frame.width,
            height: frame.height,
            background: frame.background,
            borderRadius: 24,
            border: '1px solid var(--line-strong)',
            overflow: 'hidden',
            transform: fit !== 1 ? `scale(${fit})` : undefined,
            transformOrigin: 'top left',
          }}
        >
          {frame.nodes.map((n) => <PlayNode key={n.id} node={n} />)}
        </div>
      </div>
    </div>
  );
}
