import { strFromU8, strToU8, unzipSync, zipSync } from 'fflate';
import type { ProjectDoc } from '../model/types';
import { deserializeDoc, serializeDoc } from '../model/validate';
import { loadAssetBlob } from '../storage/db';

/** プロジェクトを .tokage（zip: doc.json + assets/*.glb）に書き出す */
export async function exportTokage(doc: ProjectDoc): Promise<Blob> {
  const files: Record<string, Uint8Array> = {
    'doc.json': strToU8(serializeDoc(doc)),
  };
  for (const asset of doc.assets) {
    const buf = await loadAssetBlob(asset.id);
    if (buf) files[`assets/${asset.id}.glb`] = new Uint8Array(buf);
  }
  const zipped = zipSync(files, { level: 6 });
  return new Blob([zipped.buffer as ArrayBuffer], { type: 'application/zip' });
}

export interface ImportedProject {
  doc: ProjectDoc;
  blobs: { assetId: string; data: ArrayBuffer }[];
}

/** .tokage ファイルを読み込む */
export async function importTokage(file: File): Promise<ImportedProject> {
  const data = new Uint8Array(await file.arrayBuffer());
  const entries = unzipSync(data);
  const docEntry = entries['doc.json'];
  if (!docEntry) throw new Error('doc.json が含まれていません');
  const doc = deserializeDoc(strFromU8(docEntry));
  const blobs: ImportedProject['blobs'] = [];
  for (const asset of doc.assets) {
    const entry = entries[`assets/${asset.id}.glb`];
    if (entry) {
      const copy = new Uint8Array(entry.length);
      copy.set(entry);
      blobs.push({ assetId: asset.id, data: copy.buffer });
    }
  }
  return { doc, blobs };
}

export function downloadBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
}

export function downloadArrayBuffer(data: ArrayBuffer, filename: string): void {
  downloadBlob(new Blob([data], { type: 'model/gltf-binary' }), filename);
}
