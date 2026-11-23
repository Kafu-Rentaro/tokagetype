import { newId } from '../model/factory';
import { cloneDoc } from '../model/mutate';
import { loadAssetBlob, saveAssetBlob } from '../storage/db';
import { useEditorStore } from '../store/editorStore';
import { invalidateAssetUrl } from '../three/assetUrls';
import { downloadArrayBuffer } from './tokage';

/** GLB ファイルをアセットとして登録し、assetId を返す */
export async function addAssetFromFile(file: File): Promise<string | null> {
  const state = useEditorStore.getState();
  const doc = state.doc;
  if (!doc) return null;
  const data = await file.arrayBuffer();
  const assetId = newId('asset');
  await saveAssetBlob(assetId, data);
  const next = cloneDoc(doc);
  next.assets.push({ id: assetId, name: file.name, kind: 'glb' });
  state.commit(next);
  return assetId;
}

/** 既存アセットを view3d ノードに割り当てる */
export function assignAssetToNode(frameId: string, nodeId: string, assetId: string): void {
  const state = useEditorStore.getState();
  const doc = state.doc;
  if (!doc) return;
  const next = cloneDoc(doc);
  const frame = next.frames.find((f) => f.id === frameId);
  const node = frame?.nodes.find((n) => n.id === nodeId);
  if (node && node.type === 'view3d') {
    node.assetId = assetId;
    state.commit(next);
  }
}

/** アセットを .glb ファイルとしてダウンロードする（段階4: GLB 書き出し） */
export async function downloadAssetGLB(assetId: string, name?: string): Promise<boolean> {
  const buf = await loadAssetBlob(assetId);
  if (!buf) return false;
  downloadArrayBuffer(buf, name ?? 'model.glb');
  return true;
}

export { invalidateAssetUrl };
