import { Component, Suspense, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import * as THREE from 'three';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { OrbitControls, useGLTF } from '@react-three/drei';
import type { OrbitControls as OrbitControlsImpl } from 'three-stdlib';
import type { CameraDef } from '../model/types';
import { registerCameraSnapshot } from './cameraRegistry';
import { fitTransform } from './gltfUtils';
import { useAssetState } from './assetUrls';

interface CameraRigProps {
  nodeId: string;
  camera: CameraDef;
  /** 再生モード: カメラ変更を補間アニメーションする */
  animate: boolean;
  /** 同一カメラへの再切替でもアニメーションさせるためのキー */
  animKey: number;
  interactive: boolean;
  autoRotate: boolean;
}

function CameraRig({ nodeId, camera, animate, animKey, interactive, autoRotate }: CameraRigProps) {
  const threeCamera = useThree((s) => s.camera);
  const controls = useThree((s) => s.controls) as OrbitControlsImpl | null;
  const goalPos = useRef(new THREE.Vector3(...camera.position));
  const goalTarget = useRef(new THREE.Vector3(...camera.target));
  const animating = useRef(false);
  const first = useRef(true);

  // カメラプリセットが変わったら、再生時はゴールを更新（補間）、編集時は即座にスナップ
  // controls はマウント後に非同期で届くため依存に含め、届いた時点で注視点を反映する
  useEffect(() => {
    goalPos.current.set(...camera.position);
    goalTarget.current.set(...camera.target);
    if (animate && !first.current) {
      animating.current = true;
    } else {
      threeCamera.position.set(...camera.position);
      if (controls) {
        controls.target.set(...camera.target);
        controls.update();
      } else {
        threeCamera.lookAt(goalTarget.current);
      }
    }
    first.current = false;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [camera.id, camera.position.join(','), camera.target.join(','), animKey, controls]);

  useFrame((_, delta) => {
    if (!animating.current) return;
    const lambda = 5.5; // 約0.6秒で収束するイージング
    threeCamera.position.x = THREE.MathUtils.damp(threeCamera.position.x, goalPos.current.x, lambda, delta);
    threeCamera.position.y = THREE.MathUtils.damp(threeCamera.position.y, goalPos.current.y, lambda, delta);
    threeCamera.position.z = THREE.MathUtils.damp(threeCamera.position.z, goalPos.current.z, lambda, delta);
    if (controls) {
      controls.target.x = THREE.MathUtils.damp(controls.target.x, goalTarget.current.x, lambda, delta);
      controls.target.y = THREE.MathUtils.damp(controls.target.y, goalTarget.current.y, lambda, delta);
      controls.target.z = THREE.MathUtils.damp(controls.target.z, goalTarget.current.z, lambda, delta);
      controls.update();
    } else {
      threeCamera.lookAt(goalTarget.current);
    }
    if (threeCamera.position.distanceTo(goalPos.current) < 0.005) {
      animating.current = false;
    }
  });

  // 現在視点スナップショットの登録（「現在の視点を保存」用）
  useEffect(() => {
    return registerCameraSnapshot(nodeId, () => ({
      position: [
        Math.round(threeCamera.position.x * 1000) / 1000,
        Math.round(threeCamera.position.y * 1000) / 1000,
        Math.round(threeCamera.position.z * 1000) / 1000,
      ],
      target: controls
        ? [
            Math.round(controls.target.x * 1000) / 1000,
            Math.round(controls.target.y * 1000) / 1000,
            Math.round(controls.target.z * 1000) / 1000,
          ]
        : [0, 0.8, 0],
    }));
  }, [nodeId, threeCamera, controls]);

  return (
    <OrbitControls
      makeDefault
      enabled={interactive}
      enablePan={interactive}
      autoRotate={autoRotate}
      autoRotateSpeed={1.2}
      target={camera.target}
    />
  );
}

function GLBModel({ url, onLoaded }: { url: string; onLoaded: () => void }) {
  const gltf = useGLTF(url);
  useEffect(() => {
    onLoaded();
  }, [onLoaded, url]);
  const { scene, scale, offset } = useMemo(() => {
    const cloned = gltf.scene.clone(true);
    const fit = fitTransform(cloned);
    return { scene: cloned, scale: fit.scale, offset: fit.offset };
  }, [gltf]);
  return (
    <group position={[offset.x, offset.y, offset.z]} scale={scale}>
      <primitive object={scene} />
    </group>
  );
}

function PlaceholderModel() {
  return (
    <group position={[0, 0.8, 0]}>
      <mesh>
        <boxGeometry args={[0.75, 1.6, 0.08]} />
        <meshStandardMaterial color="#7a7d85" wireframe />
      </mesh>
    </group>
  );
}

/**
 * GLB 読み込みエラーでアプリ全体が落ちないようにするためのエラー境界。
 * useGLTF は読み込み失敗時に throw する。境界が無いと React ツリー全体が
 * アンマウントされ白画面になる（フェーズ5監査で確認した不具合）。
 */
class ModelErrorBoundary extends Component<
  { onError: (error: Error) => void; fallback: ReactNode; children: ReactNode },
  { error: Error | null }
> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error): { error: Error } {
    return { error };
  }

  componentDidCatch(error: Error): void {
    this.props.onError(error);
  }

  render(): ReactNode {
    return this.state.error ? this.props.fallback : this.props.children;
  }
}

export interface View3DCanvasProps {
  nodeId: string;
  assetId: string | null;
  camera: CameraDef;
  animate?: boolean;
  animKey?: number;
  interactive?: boolean;
  autoRotate?: boolean;
}

type LoadPhase = 'idle' | 'loading' | 'ready' | 'error';

/** View3DNode の中身として重ねる R3F キャンバス（読み込み状態の表示とエラー隔離つき） */
export default function View3DCanvas({
  nodeId,
  assetId,
  camera,
  animate = false,
  animKey = 0,
  interactive = false,
  autoRotate = false,
}: View3DCanvasProps) {
  const asset = useAssetState(assetId);
  const assetUrl = asset.url;
  const [phase, setPhase] = useState<LoadPhase>('idle');
  const [errorMsg, setErrorMsg] = useState('');
  const [retry, setRetry] = useState(0);

  // URL が変わる / 再試行するたびに読み込み状態をリセットする
  useEffect(() => {
    setPhase(assetUrl ? 'loading' : 'idle');
    setErrorMsg('');
  }, [assetUrl, retry]);

  const handleLoaded = useCallback(() => setPhase('ready'), []);
  const handleError = useCallback((e: Error) => {
    setPhase('error');
    setErrorMsg(e.message || String(e));
  }, []);

  const doRetry = useCallback(() => {
    if (assetUrl) useGLTF.clear(assetUrl);
    setRetry((r) => r + 1);
  }, [assetUrl]);

  const showLoading =
    asset.status === 'loading' || (assetUrl !== null && phase === 'loading');

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%' }}>
      <Canvas
        camera={{ position: camera.position, fov: 40, near: 0.01, far: 100 }}
        gl={{ antialias: true, alpha: true }}
        style={{ width: '100%', height: '100%', display: 'block' }}
      >
        <ambientLight intensity={0.9} />
        <directionalLight position={[3, 5, 4]} intensity={1.6} />
        <directionalLight position={[-3, 2, -4]} intensity={0.5} />
        <gridHelper args={[6, 12, '#c9d4e6', '#e4eaf4']} position={[0, 0, 0]} />
        <ModelErrorBoundary
          key={`${assetUrl ?? 'none'}#${retry}`}
          onError={handleError}
          fallback={<PlaceholderModel />}
        >
          <Suspense fallback={null}>
            {assetUrl ? (
              <GLBModel url={assetUrl} onLoaded={handleLoaded} />
            ) : (
              <PlaceholderModel />
            )}
          </Suspense>
        </ModelErrorBoundary>
        <CameraRig
          nodeId={nodeId}
          camera={camera}
          animate={animate}
          animKey={animKey}
          interactive={interactive}
          autoRotate={autoRotate}
        />
      </Canvas>

      {asset.status === 'missing' && (
        <div style={{ ...overlayStyle, background: 'rgba(221,225,230,0.88)' }}>
          アセットの実体が見つかりません
          <span style={overlaySubStyle}>ストレージに GLB が保存されていません</span>
        </div>
      )}
      {phase === 'error' && (
        <div style={{ ...overlayStyle, background: 'rgba(221,225,230,0.88)' }}>
          GLB の読み込みに失敗しました
          <span style={overlaySubStyle}>{errorMsg}</span>
          <button
            onClick={doRetry}
            style={{
              pointerEvents: 'auto',
              marginTop: 6,
              background: 'var(--panel, #1e1f23)',
              color: 'var(--text, #e8e9ec)',
              border: '1px solid var(--line-strong, #55565e)',
              borderRadius: 4,
              padding: '4px 12px',
              cursor: 'pointer',
              fontSize: 12,
              fontFamily: 'inherit',
            }}
          >
            再試行
          </button>
        </div>
      )}
      {phase !== 'error' && asset.status !== 'missing' && showLoading && (
        <div style={{ ...overlayStyle, pointerEvents: 'none' }}>読み込み中…</div>
      )}
      {!assetId && (
        <div style={{ ...overlayStyle, pointerEvents: 'none' }}>
          GLB 未設定
          <span style={overlaySubStyle}>ファイルを選択 / ドロップで割り当て</span>
        </div>
      )}
    </div>
  );
}

const overlayStyle: React.CSSProperties = {
  position: 'absolute',
  inset: 0,
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 2,
  color: '#5a5e66',
  fontSize: 13,
  textAlign: 'center',
  padding: 8,
  pointerEvents: 'none',
};

const overlaySubStyle: React.CSSProperties = {
  fontSize: 11,
  color: '#8a8f98',
  maxWidth: '92%',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  display: '-webkit-box',
  WebkitLineClamp: 2,
  WebkitBoxOrient: 'vertical',
};
