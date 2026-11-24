import { openDB, type IDBPDatabase } from 'idb';
import type { ProjectDoc } from '../model/types';

const DB_NAME = 'tokagetype';
const DB_VERSION = 1;

let dbPromise: Promise<IDBPDatabase> | null = null;

function getDB(): Promise<IDBPDatabase> {
  if (!dbPromise) {
    dbPromise = openDB(DB_NAME, DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains('projects')) db.createObjectStore('projects');
        if (!db.objectStoreNames.contains('assetBlobs')) db.createObjectStore('assetBlobs');
      },
    });
  }
  return dbPromise;
}

export async function saveProject(doc: ProjectDoc): Promise<void> {
  const db = await getDB();
  await db.put('projects', doc, doc.id);
}

export async function loadProject(id: string): Promise<ProjectDoc | undefined> {
  const db = await getDB();
  return (await db.get('projects', id)) as ProjectDoc | undefined;
}

export async function listProjects(): Promise<ProjectDoc[]> {
  const db = await getDB();
  return (await db.getAll('projects')) as ProjectDoc[];
}

export async function deleteProject(id: string): Promise<void> {
  const db = await getDB();
  await db.delete('projects', id);
}

export async function saveAssetBlob(assetId: string, data: ArrayBuffer): Promise<void> {
  const db = await getDB();
  await db.put('assetBlobs', data, assetId);
}

export async function loadAssetBlob(assetId: string): Promise<ArrayBuffer | undefined> {
  const db = await getDB();
  return (await db.get('assetBlobs', assetId)) as ArrayBuffer | undefined;
}

/** 保存済みアセット実体の ID 一覧（起動時の欠落チェック用） */
export async function listAssetBlobIds(): Promise<string[]> {
  const db = await getDB();
  return (await db.getAllKeys('assetBlobs')) as string[];
}

export async function deleteAssetBlob(assetId: string): Promise<void> {
  const db = await getDB();
  await db.delete('assetBlobs', assetId);
}
