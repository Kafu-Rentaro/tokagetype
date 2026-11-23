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
