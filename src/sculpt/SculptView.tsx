import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';
import { Canvas, type ThreeEvent } from '@react-three/fiber';
import { OrbitControls } from '@react-three/drei';
import { acceleratedRaycast } from 'three-mesh-bvh';
import type { View3DNode } from '../model/types';
import { newId } from '../model/factory';
import { loadAssetBlob, saveAssetBlob } from '../storage/db';
import { invalidateAssetUrl } from '../three/assetUrls';
import { buildSphereGLB, exportToGLB, parseGLB } from '../three/gltfUtils';
import { applyBrush, ensureBVH, type BrushMode } from './brush';
import { useEditorStore } from '../store/editorStore';

interface SculptViewProps {
  node: View3DNode;
  onClose: () => void;
}

const MAX_UNDO = 30;

export default function SculptView({ node, onClose }: SculptViewProps) {
  const commit = useEditorStore((s) => s.commit);
  const doc = useEditorStore((s) => s.doc);
  const frameId = useEditorStore((s) => s.currentFrameId);

  const [geometry, setGeometry] = useState<THREE.BufferGeometry | null>(null);
  const [material, setMaterial] = useState<THREE.Material | null>(null);
  /** 書き出し時に他メッシュを保持するための元シーンと、対象メッシュの順序・逆行列 */
  const sourceRef = useRef<{
    scene: THREE.Object3D;
    meshIndex: number;
    worldInverse: THREE.Matrix4;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [brush, setBrush] = useState<BrushMode>('pull');
  const [radius, setRadius] = useState(0.25);
  const [strength, setStrength] = useState(0.6);
  const [rotateMode, setRotateMode] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);

  const meshRef = useRef<THREE.Mesh | null>(null);
  const brushing = useRef(false);
  const undoStack = useRef<Float32Array[]>([]);
  const redoStack = useRef<Float32Array[]>([]);
  const [, forceRender] = useState(0);

  // 対象ジオメトリの読み込み（GLB の最初のメッシュ、無ければ球）
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        let buffer: ArrayBuffer | undefined;
        if (node.assetId) buffer = await loadAssetBlob(node.assetId);
        if (!buffer) buffer = await buildSphereGLB();
        const gltf = await parseGLB(buffer);
        gltf.scene.updateMatrixWorld(true);
        const meshes: THREE.Mesh[] = [];
        gltf.scene.traverse((o) => {
          if ((o as THREE.Mesh).isMesh) meshes.push(o as THREE.Mesh);
        });
        const found = meshes[0];
        if (!found) throw new Error('GLB 内にメッシュが見つかりません');
        const mesh = found;
        sourceRef.current = {
          scene: gltf.scene,
          meshIndex: 0,
          worldInverse: mesh.matrixWorld.clone().invert(),
        };
        const geo = mesh.geometry.clone();
        geo.applyMatrix4(mesh.matrixWorld);
        if (!geo.getAttribute('normal')) geo.computeVertexNormals();
        ensureBVH(geo);
        if (!cancelled) {
          setGeometry(geo);
          const mat = (mesh.material as THREE.Material) ?? new THREE.MeshStandardMaterial({ color: '#b8c4d4' });
          setMaterial(Array.isArray(mat) ? mat[0] : mat);
        }
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node.assetId]);

  const snapshot = useCallback((): Float32Array | null => {
    if (!geometry) return null;
    return new Float32Array(geometry.getAttribute('position').array);
  }, [geometry]);

  const restore = useCallback(
    (data: Float32Array) => {
      if (!geometry) return;
      const attr = geometry.getAttribute('position');
      (attr.array as Float32Array).set(data);
      attr.needsUpdate = true;
      geometry.computeVertexNormals();
      geometry.computeBoundingSphere();
      forceRender((t) => t + 1);
    },
    [geometry],
  );

  const undo = useCallback(() => {
    const prev = undoStack.current.pop();
    const current = snapshot();
    if (!prev || !current) return;
    redoStack.current.push(current);
    restore(prev);
  }, [restore, snapshot]);

  const redo = useCallback(() => {
    const next = redoStack.current.pop();
    const current = snapshot();
    if (!next || !current) return;
    undoStack.current.push(current);
    restore(next);
  }, [restore, snapshot]);

  const handlePointerDown = (e: ThreeEvent<PointerEvent>) => {
    if (rotateMode || !geometry) return;
    e.stopPropagation();
    const snap = snapshot();
    if (snap) {
      undoStack.current.push(snap);
      if (undoStack.current.length > MAX_UNDO) undoStack.current.shift();
      redoStack.current = [];
    }
    brushing.current = true;
    (e.target as Element).setPointerCapture?.(e.pointerId);
    applyBrush(geometry, e.point, radius, strength, brush);
    setDirty(true);
    forceRender((t) => t + 1);
  };

  const handlePointerMove = (e: ThreeEvent<PointerEvent>) => {
    if (!brushing.current || !geometry) return;
    applyBrush(geometry, e.point, radius, strength * 0.5, brush);
    forceRender((t) => t + 1);
  };

  const handlePointerUp = () => {
    brushing.current = false;
  };

  /** 造形結果を GLB 化し、アセットを置き換えて保存する */
  const applyAndSave = async () => {
    if (!geometry || !doc || !frameId) return;
    setSaving(true);
    try {
      // 元シーンを複製し、対象メッシュのジオメトリだけを造形結果（ローカル空間へ戻したもの）に差し替える。
      // これにより画面・レンズなど他のメッシュも保持される。
      let exportRoot: THREE.Object3D;
      const source = sourceRef.current;
      if (source) {
        const sceneClone = source.scene.clone(true);
        const localGeo = geometry.clone().applyMatrix4(source.worldInverse);
        const meshes: THREE.Mesh[] = [];
        sceneClone.traverse((o) => {
          if ((o as THREE.Mesh).isMesh) meshes.push(o as THREE.Mesh);
        });
        const target = meshes[source.meshIndex];
        if (target) target.geometry = localGeo;
        exportRoot = sceneClone;
      } else {
        const mat = material ?? new THREE.MeshStandardMaterial({ color: '#b8c4d4', roughness: 0.6 });
        const mesh = new THREE.Mesh(geometry.clone(), mat);
        mesh.name = 'SculptedMesh';
        exportRoot = mesh;
      }
      const glb = await exportToGLB(exportRoot);

      let assetId = node.assetId;
      const next = JSON.parse(JSON.stringify(doc)) as typeof doc;
      if (!assetId) {
        assetId = newId('asset');
        next.assets.push({ id: assetId, name: 'sculpted.glb', kind: 'glb' });
      } else {
        const ref = next.assets.find((a) => a.id === assetId);
        if (ref && !ref.name.includes('(sculpt)')) ref.name = `${ref.name} (sculpt)`;
      }
      const frame = next.frames.find((f) => f.id === frameId);
      const target = frame?.nodes.find((n) => n.id === node.id);
      if (target && target.type === 'view3d') target.assetId = assetId;

      await saveAssetBlob(assetId, glb);
      invalidateAssetUrl(assetId);
      commit(next);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setSaving(false);
    }
  };

  const brushCursor = useMemo(() => (rotateMode ? 'grab' : 'crosshair'), [rotateMode]);

  return (
    <div style={styles.overlay}>
      <div style={styles.header}>
        <strong>スカルプト: {node.name}</strong>
        <span style={{ color: '#9fb2cc', fontSize: 12 }}>
          頂点数は固定です（リメッシュなし）。1ストローク = 1 Undo。
        </span>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 8 }}>
          <button style={styles.btn} onClick={undo} disabled={undoStack.current.length === 0}>↩ Undo</button>
          <button style={styles.btn} onClick={redo} disabled={redoStack.current.length === 0}>↪ Redo</button>
          <button style={styles.btn} onClick={onClose}>閉じる</button>
          <button style={{ ...styles.btn, ...styles.primary }} onClick={applyAndSave} disabled={!dirty || saving}>
            {saving ? '保存中…' : '適用して保存（GLB化）'}
          </button>
        </div>
      </div>

      <div style={styles.toolbar}>
        <label>
          ブラシ:
          <select value={brush} onChange={(e) => setBrush(e.target.value as BrushMode)} style={styles.select}>
            <option value="pull">pull（引き出す）</option>
            <option value="push">push（押し込む）</option>
            <option value="smooth">smooth（滑らか）</option>
          </select>
        </label>
        <label>
          半径 {radius.toFixed(2)}
          <input type="range" min={0.05} max={0.8} step={0.01} value={radius} onChange={(e) => setRadius(Number(e.target.value))} />
        </label>
        <label>
          強さ {strength.toFixed(2)}
          <input type="range" min={0.05} max={1.5} step={0.05} value={strength} onChange={(e) => setStrength(Number(e.target.value))} />
        </label>
        <label>
          <input type="checkbox" checked={rotateMode} onChange={(e) => setRotateMode(e.target.checked)} />
          視点回転モード（ブラシ無効）
        </label>
        {error && <span style={{ color: '#ff9c9c' }}>{error}</span>}
      </div>

      <div style={{ flex: 1, cursor: brushCursor }}>
        {geometry && material ? (
          <Canvas camera={{ position: [0, 0.9, 3.2], fov: 40, near: 0.01, far: 100 }}>
            <ambientLight intensity={0.9} />
            <directionalLight position={[3, 5, 4]} intensity={1.6} />
            <directionalLight position={[-3, 2, -4]} intensity={0.5} />
            <gridHelper args={[6, 12, '#3a4a66', '#2a3850']} />
            <mesh
              ref={(m) => {
                meshRef.current = m;
                if (m) m.raycast = acceleratedRaycast;
              }}
              geometry={geometry}
              material={material}
              onPointerDown={handlePointerDown}
              onPointerMove={handlePointerMove}
              onPointerUp={handlePointerUp}
            />
            <OrbitControls makeDefault enabled={rotateMode || !geometry} enablePan={rotateMode} target={[0, 0.8, 0]} />
          </Canvas>
        ) : (
          <div style={{ color: '#dbe6f5', padding: 24 }}>{error ?? '読み込み中…'}</div>
        )}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  overlay: {
    position: 'fixed', inset: 0, zIndex: 100, background: '#141c2c',
    display: 'flex', flexDirection: 'column',
  },
  header: {
    display: 'flex', alignItems: 'center', gap: 12, padding: '10px 14px',
    color: '#eef3fa', borderBottom: '1px solid #2a3850',
  },
  toolbar: {
    display: 'flex', alignItems: 'center', gap: 18, padding: '8px 14px',
    color: '#dbe6f5', fontSize: 13, borderBottom: '1px solid #2a3850', flexWrap: 'wrap',
  },
  btn: {
    background: '#243350', color: '#e6eefb', border: '1px solid #3a4f7a',
    borderRadius: 6, padding: '6px 12px', cursor: 'pointer', fontSize: 13,
  },
  primary: { background: '#2f6fd0', borderColor: '#2f6fd0' },
  select: { marginLeft: 6, background: '#0f1626', color: '#e6eefb', border: '1px solid #3a4f7a', borderRadius: 4 },
};
