import type { Action, Frame, Node, ProjectDoc, View3DNode } from './types';

export interface ValidationIssue {
  level: 'error' | 'warning';
  message: string;
  nodeId?: string;
  frameId?: string;
}

/** 全フレームからノードを検索する */
export function findNode(doc: ProjectDoc, nodeId: string): { frame: Frame; node: Node } | null {
  for (const frame of doc.frames) {
    const node = frame.nodes.find((n) => n.id === nodeId);
    if (node) return { frame, node };
  }
  return null;
}

export function findFrame(doc: ProjectDoc, frameId: string): Frame | null {
  return doc.frames.find((f) => f.id === frameId) ?? null;
}

/** Action の参照先が実在するか検証し、問題があれば issue を返す */
export function validateAction(doc: ProjectDoc, action: Action): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  switch (action.type) {
    case 'setCamera': {
      const found = findNode(doc, action.viewNodeId);
      if (!found || found.node.type !== 'view3d') {
        issues.push({ level: 'warning', message: `setCamera の参照先 3Dビュー (${action.viewNodeId}) が存在しません` });
      } else if (!(found.node as View3DNode).cameras.some((c) => c.id === action.cameraId)) {
        issues.push({ level: 'warning', message: `setCamera のカメラ (${action.cameraId}) が存在しません` });
      }
      break;
    }
    case 'navigate':
      if (!findFrame(doc, action.frameId)) {
        issues.push({ level: 'warning', message: `navigate の遷移先フレーム (${action.frameId}) が存在しません` });
      }
      break;
    case 'toggleVisible':
      if (!findNode(doc, action.nodeId)) {
        issues.push({ level: 'warning', message: `toggleVisible の対象ノード (${action.nodeId}) が存在しません` });
      }
      break;
    case 'openUrl':
      if (!/^https?:\/\//.test(action.url)) {
        issues.push({ level: 'warning', message: `openUrl の URL が不正です: ${action.url}` });
      }
      break;
  }
  return issues;
}

/** 文書全体の検証。ID の一意性、参照整合性、必須フィールドを見る */
export function validateDoc(doc: ProjectDoc): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  if (doc.version !== 1) {
    issues.push({ level: 'error', message: `未対応の文書バージョンです: ${String(doc.version)}` });
  }
  if (doc.frames.length === 0) {
    issues.push({ level: 'error', message: 'フレームが1つもありません' });
  }
  if (!findFrame(doc, doc.startFrameId)) {
    issues.push({ level: 'error', message: `開始フレーム (${doc.startFrameId}) が存在しません` });
  }

  const ids = new Set<string>();
  const dup = (id: string, what: string) => {
    if (ids.has(id)) issues.push({ level: 'error', message: `${what} の ID が重複しています: ${id}` });
    ids.add(id);
  };
  dup(doc.id, 'プロジェクト');
  for (const asset of doc.assets) dup(asset.id, 'アセット');
  for (const frame of doc.frames) {
    dup(frame.id, 'フレーム');
    if (!(frame.width > 0) || !(frame.height > 0)) {
      issues.push({ level: 'error', message: `フレーム ${frame.name} のサイズが不正です`, frameId: frame.id });
    }
    for (const node of frame.nodes) {
      dup(node.id, 'ノード');
      if (node.type === 'view3d') {
        for (const cam of node.cameras) dup(cam.id, 'カメラ');
        if (!node.cameras.some((c) => c.id === node.activeCameraId)) {
          issues.push({ level: 'warning', message: `3Dビュー ${node.name} のアクティブカメラが存在しません`, nodeId: node.id, frameId: frame.id });
        }
        if (node.assetId && !doc.assets.some((a) => a.id === node.assetId)) {
          issues.push({ level: 'warning', message: `3Dビュー ${node.name} のアセット (${node.assetId}) が存在しません`, nodeId: node.id, frameId: frame.id });
        }
      }
      for (const action of node.actions ?? []) {
        for (const issue of validateAction(doc, action)) {
          issues.push({ ...issue, nodeId: node.id, frameId: frame.id });
        }
      }
    }
  }
  return issues;
}

/** JSON 文字列化（保存・書き出し用） */
export function serializeDoc(doc: ProjectDoc): string {
  return JSON.stringify(doc);
}

/** JSON から文書を復元する。構造が壊れていれば例外 */
export function deserializeDoc(json: string): ProjectDoc {
  const parsed = JSON.parse(json) as ProjectDoc;
  if (typeof parsed !== 'object' || parsed === null) throw new Error('文書がオブジェクトではありません');
  if (parsed.version !== 1) throw new Error(`未対応の文書バージョンです: ${String(parsed.version)}`);
  if (!Array.isArray(parsed.frames) || !Array.isArray(parsed.assets)) {
    throw new Error('文書の構造が不正です（frames / assets が配列ではありません）');
  }
  return parsed;
}
