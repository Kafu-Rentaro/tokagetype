import * as THREE from 'three';
import { computeBoundsTree, disposeBoundsTree } from 'three-mesh-bvh';

export type BrushMode = 'push' | 'pull' | 'smooth';

type BVHGeometry = THREE.BufferGeometry & {
  computeBoundsTree?: (options?: Record<string, unknown>) => unknown;
  disposeBoundsTree?: () => void;
  boundsTree?: unknown;
};

/** three-mesh-bvh の bounds tree を geometry に用意する */
export function ensureBVH(geometry: THREE.BufferGeometry): void {
  const g = geometry as BVHGeometry;
  if (!g.computeBoundsTree) {
    g.computeBoundsTree = computeBoundsTree as unknown as BVHGeometry['computeBoundsTree'];
  }
  if (!g.disposeBoundsTree) {
    g.disposeBoundsTree = disposeBoundsTree as unknown as BVHGeometry['disposeBoundsTree'];
  }
  g.computeBoundsTree();
}

export function rebuildBVH(geometry: THREE.BufferGeometry): void {
  const g = geometry as BVHGeometry;
  if (g.computeBoundsTree) g.computeBoundsTree();
}

/** BVH の shapecast で球範囲と交差する三角形の頂点インデックスを集める */
export function findVerticesInRadius(
  geometry: THREE.BufferGeometry,
  center: THREE.Vector3,
  radius: number,
): number[] {
  const tree = (geometry as unknown as { boundsTree?: { shapecast: (c: unknown) => void } }).boundsTree;
  const position = geometry.getAttribute('position');
  const result = new Set<number>();
  const sphere = new THREE.Sphere(center, radius);
  const v = new THREE.Vector3();
  const collect = (triIndex: number) => {
    const index = geometry.getIndex();
    for (let k = 0; k < 3; k++) {
      const vi = index ? index.getX(triIndex * 3 + k) : triIndex * 3 + k;
      v.fromBufferAttribute(position, vi);
      if (v.distanceTo(center) <= radius) result.add(vi);
    }
  };
  if (!tree) {
    // フォールバック: 全頂点走査
    for (let i = 0; i < position.count; i++) {
      v.fromBufferAttribute(position, i);
      if (v.distanceTo(center) <= radius) result.add(i);
    }
    return [...result];
  }
  tree.shapecast({
    intersectsBounds: (box: THREE.Box3) => box.intersectsSphere(sphere),
    intersectsTriangle: (_tri: unknown, triIndex: number) => {
      collect(triIndex);
      return false;
    },
  });
  return [...result];
}

/**
 * ブラシを1回適用する。center は geometry のローカル座標。
 * - pull: 法線方向へ引き出す（膨らませる）
 * - push: 法線と逆方向へ押し込む（へこませる）
 * - smooth: 範囲内頂点の平均位置へ寄せる
 */
export function applyBrush(
  geometry: THREE.BufferGeometry,
  center: THREE.Vector3,
  radius: number,
  strength: number,
  mode: BrushMode,
): number {
  const position = geometry.getAttribute('position');
  const normal = geometry.getAttribute('normal');
  const indices = findVerticesInRadius(geometry, center, radius);
  if (indices.length === 0) return 0;

  const v = new THREE.Vector3();
  const n = new THREE.Vector3();

  if (mode === 'smooth') {
    const avg = new THREE.Vector3();
    for (const i of indices) {
      v.fromBufferAttribute(position, i);
      avg.add(v);
    }
    avg.divideScalar(indices.length);
    for (const i of indices) {
      v.fromBufferAttribute(position, i);
      const d = v.distanceTo(center);
      const falloff = Math.max(0, 1 - d / radius);
      v.lerp(avg, Math.min(1, strength * falloff));
      position.setXYZ(i, v.x, v.y, v.z);
    }
  } else {
    const dirSign = mode === 'pull' ? 1 : -1;
    for (const i of indices) {
      v.fromBufferAttribute(position, i);
      const d = v.distanceTo(center);
      const falloff = Math.max(0, 1 - d / radius);
      const smoothFalloff = falloff * falloff * (3 - 2 * falloff);
      if (normal) n.fromBufferAttribute(normal, i);
      else n.copy(v).sub(center).normalize();
      const amount = strength * smoothFalloff * dirSign * radius * 0.35;
      v.addScaledVector(n, amount);
      position.setXYZ(i, v.x, v.y, v.z);
    }
  }

  position.needsUpdate = true;
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  rebuildBVH(geometry);
  return indices.length;
}
