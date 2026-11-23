import type { CameraDef } from '../model/types';

/**
 * 表示中の R3F カメラの現在視点を取得するためのレジストリ。
 * プロパティパネルの「現在の視点をカメラとして保存」で使う。
 */
type SnapshotFn = () => Omit<CameraDef, 'id' | 'name'> | null;

const registry = new Map<string, SnapshotFn>();

export function registerCameraSnapshot(nodeId: string, fn: SnapshotFn): () => void {
  registry.set(nodeId, fn);
  return () => {
    if (registry.get(nodeId) === fn) registry.delete(nodeId);
  };
}

export function getCameraSnapshot(nodeId: string): Omit<CameraDef, 'id' | 'name'> | null {
  return registry.get(nodeId)?.() ?? null;
}
