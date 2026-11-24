import { useEffect, useState } from 'react';
import { loadAssetBlob } from '../storage/db';

/** assetId → Blob URL のキャッシュ。アセット置換時に invalidate する */
const cache = new Map<string, string>();
const listeners = new Set<() => void>();

function emit() {
  for (const fn of listeners) fn();
}

export async function getAssetUrl(assetId: string): Promise<string | null> {
  const hit = cache.get(assetId);
  if (hit) return hit;
  const buf = await loadAssetBlob(assetId);
  if (!buf) return null;
  const url = URL.createObjectURL(new Blob([buf], { type: 'model/gltf-binary' }));
  cache.set(assetId, url);
  return url;
}

export function invalidateAssetUrl(assetId: string): void {
  const url = cache.get(assetId);
  if (url) URL.revokeObjectURL(url);
  cache.delete(assetId);
  emit();
}

export type AssetLoadState =
  | { status: 'idle'; url: null } // 未割当
  | { status: 'loading'; url: null } // Blob 読み込み中
  | { status: 'ready'; url: string } // URL 取得済み
  | { status: 'missing'; url: null }; // assetId はあるが実体がストレージに無い

/** アセットの読み込み状態（未割当 / 読込中 / 準備完了 / 実体なし）を購読するフック */
export function useAssetState(assetId: string | null): AssetLoadState {
  const [state, setState] = useState<AssetLoadState>({ status: 'idle', url: null });
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const fn = () => setTick((t) => t + 1);
    listeners.add(fn);
    return () => {
      listeners.delete(fn);
    };
  }, []);

  // tick を依存に含める: invalidate（アセット置換）時に URL を取り直す。
  // 含めないと失効済み Blob URL を使い続けて読み込みエラーになる。
  useEffect(() => {
    let cancelled = false;
    if (!assetId) {
      setState({ status: 'idle', url: null });
      return;
    }
    const hit = cache.get(assetId);
    if (hit) {
      setState({ status: 'ready', url: hit });
      return;
    }
    setState({ status: 'loading', url: null });
    void getAssetUrl(assetId).then((u) => {
      if (cancelled) return;
      setState(u ? { status: 'ready', url: u } : { status: 'missing', url: null });
    });
    return () => {
      cancelled = true;
    };
  }, [assetId, tick]);

  return state;
}

/** アセットの Blob URL を購読するフック */
export function useAssetUrl(assetId: string | null): string | null {
  const [url, setUrl] = useState<string | null>(null);
  const [, setTick] = useState(0);

  useEffect(() => {
    const fn = () => setTick((t) => t + 1);
    listeners.add(fn);
    return () => {
      listeners.delete(fn);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    if (!assetId) {
      setUrl(null);
      return;
    }
    const hit = cache.get(assetId);
    if (hit) {
      setUrl(hit);
      return;
    }
    void getAssetUrl(assetId).then((u) => {
      if (!cancelled) setUrl(u);
    });
    return () => {
      cancelled = true;
    };
  }, [assetId]);

  return url;
}
