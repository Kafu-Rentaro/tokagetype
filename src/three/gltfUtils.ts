import * as THREE from 'three';
import { GLTFExporter } from 'three/addons/exporters/GLTFExporter.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import type { GLTF } from 'three/addons/loaders/GLTFLoader.js';

export type { GLTF };

/** Object3D を GLB (ArrayBuffer) に書き出す */
export function exportToGLB(object: THREE.Object3D): Promise<ArrayBuffer> {
  return new Promise((resolve, reject) => {
    new GLTFExporter().parse(
      object,
      (result) => {
        if (result instanceof ArrayBuffer) resolve(result);
        else reject(new Error('GLB 書き出し結果がバイナリではありません'));
      },
      (error) => reject(error instanceof Error ? error : new Error(String(error))),
      { binary: true },
    );
  });
}

/** ArrayBuffer から GLTF を読み込む */
export async function parseGLB(buffer: ArrayBuffer): Promise<GLTF> {
  const loader = new GLTFLoader();
  return loader.parseAsync(buffer, '');
}

/**
 * サンプル用スマートフォンの GLB を three のジオメトリから生成する。
 * 本体（前面: 画面 / 背面: カメラレンズ）を持つ単純なモデル。
 * 座標系は glTF 標準（Y-up、メートル相当）。原点は底面中央。
 */
export async function buildSamplePhoneGLB(): Promise<ArrayBuffer> {
  const root = new THREE.Group();
  root.name = 'SamplePhone';

  const bodyMat = new THREE.MeshStandardMaterial({ color: '#2b3a55', roughness: 0.35, metalness: 0.6 });
  const body = new THREE.Mesh(new THREE.BoxGeometry(0.75, 1.6, 0.08), bodyMat);
  body.position.y = 0.8;
  body.name = 'Body';
  root.add(body);

  const screenMat = new THREE.MeshStandardMaterial({
    color: '#7fd4ff', roughness: 0.15, metalness: 0.1, emissive: '#1c4d66', emissiveIntensity: 0.7,
  });
  const screen = new THREE.Mesh(new THREE.BoxGeometry(0.68, 1.5, 0.005), screenMat);
  screen.position.set(0, 0.8, 0.043);
  screen.name = 'Screen';
  root.add(screen);

  const lensMat = new THREE.MeshStandardMaterial({ color: '#101418', roughness: 0.2, metalness: 0.8 });
  const lensBase = new THREE.Mesh(new THREE.CylinderGeometry(0.11, 0.11, 0.02, 32), lensMat);
  lensBase.rotation.x = Math.PI / 2;
  lensBase.position.set(-0.18, 1.38, -0.05);
  lensBase.name = 'CameraBase';
  root.add(lensBase);

  const lensGlassMat = new THREE.MeshStandardMaterial({ color: '#3a6ea5', roughness: 0.05, metalness: 0.9 });
  const lensGlass = new THREE.Mesh(new THREE.CylinderGeometry(0.06, 0.06, 0.025, 32), lensGlassMat);
  lensGlass.rotation.x = Math.PI / 2;
  lensGlass.position.set(-0.18, 1.38, -0.055);
  lensGlass.name = 'CameraLens';
  root.add(lensGlass);

  const logoMat = new THREE.MeshStandardMaterial({ color: '#d7e3f4', roughness: 0.4, metalness: 0.3 });
  const logo = new THREE.Mesh(new THREE.CylinderGeometry(0.09, 0.09, 0.006, 32), logoMat);
  logo.rotation.x = Math.PI / 2;
  logo.position.set(0, 0.75, -0.043);
  logo.name = 'BackLogo';
  root.add(logo);

  return exportToGLB(root);
}

/** スカルプト用: 細分化した球プリミティブの GLB を生成する */
export async function buildSphereGLB(): Promise<ArrayBuffer> {
  const geo = new THREE.SphereGeometry(0.8, 96, 72);
  geo.translate(0, 0.8, 0);
  const mat = new THREE.MeshStandardMaterial({ color: '#b8c4d4', roughness: 0.6, metalness: 0.05 });
  const mesh = new THREE.Mesh(geo, mat);
  mesh.name = 'SculptSphere';
  return exportToGLB(mesh);
}

/** Object3D のバウンディングボックスを [0, 0.8, 0] 付近に正規化するスケール/オフセットを返す */
export function fitTransform(object: THREE.Object3D): { scale: number; offset: THREE.Vector3 } {
  const box = new THREE.Box3().setFromObject(object);
  const size = box.getSize(new THREE.Vector3());
  const center = box.getCenter(new THREE.Vector3());
  const maxDim = Math.max(size.x, size.y, size.z) || 1;
  const scale = 1.6 / maxDim;
  const offset = new THREE.Vector3(-center.x * scale, -box.min.y * scale, -center.z * scale);
  return { scale, offset };
}
