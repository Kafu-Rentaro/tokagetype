import { Suspense, useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { OrbitControls, useGLTF } from '@react-three/drei';
import type { OrbitControls as OrbitControlsImpl } from 'three-stdlib';
import type { CameraDef } from '../model/types';
import { registerCameraSnapshot } from './cameraRegistry';
import { fitTransform } from './gltfUtils';

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

function GLBModel({ url }: { url: string }) {
  const gltf = useGLTF(url);
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

export interface View3DCanvasProps {
  nodeId: string;
  assetUrl: string | null;
  camera: CameraDef;
  animate?: boolean;
  animKey?: number;
  interactive?: boolean;
  autoRotate?: boolean;
}

/** View3DNode の中身として重ねる R3F キャンバス */
export default function View3DCanvas({
  nodeId,
  assetUrl,
  camera,
  animate = false,
  animKey = 0,
  interactive = false,
  autoRotate = false,
}: View3DCanvasProps) {
  return (
    <Canvas
      camera={{ position: camera.position, fov: 40, near: 0.01, far: 100 }}
      gl={{ antialias: true, alpha: true }}
      style={{ width: '100%', height: '100%', display: 'block' }}
    >
      <ambientLight intensity={0.9} />
      <directionalLight position={[3, 5, 4]} intensity={1.6} />
      <directionalLight position={[-3, 2, -4]} intensity={0.5} />
      <gridHelper args={[6, 12, '#c9d4e6', '#e4eaf4']} position={[0, 0, 0]} />
      <Suspense fallback={null}>
        {assetUrl ? <GLBModel url={assetUrl} /> : <PlaceholderModel />}
      </Suspense>
      <CameraRig
        nodeId={nodeId}
        camera={camera}
        animate={animate}
        animKey={animKey}
        interactive={interactive}
        autoRotate={autoRotate}
      />
    </Canvas>
  );
}
