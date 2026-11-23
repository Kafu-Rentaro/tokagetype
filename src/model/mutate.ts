import type { Frame, Node, ProjectDoc } from './types';

/** 文書のディープコピー（structuredClone が無い環境向けフォールバック付き） */
export function cloneDoc(doc: ProjectDoc): ProjectDoc {
  return JSON.parse(JSON.stringify(doc)) as ProjectDoc;
}

export function updateFrame(doc: ProjectDoc, frameId: string, patch: Partial<Frame>): ProjectDoc {
  const next = cloneDoc(doc);
  const frame = next.frames.find((f) => f.id === frameId);
  if (frame) Object.assign(frame, patch);
  return next;
}

export function updateNode(
  doc: ProjectDoc,
  frameId: string,
  nodeId: string,
  patch: Partial<Node> & Record<string, unknown>,
): ProjectDoc {
  const next = cloneDoc(doc);
  const frame = next.frames.find((f) => f.id === frameId);
  const node = frame?.nodes.find((n) => n.id === nodeId);
  if (node) Object.assign(node, patch);
  return next;
}

export function addNodeToFrame(doc: ProjectDoc, frameId: string, node: Node): ProjectDoc {
  const next = cloneDoc(doc);
  const frame = next.frames.find((f) => f.id === frameId);
  if (frame) frame.nodes.push(node);
  return next;
}

export function removeNodeFromFrame(doc: ProjectDoc, frameId: string, nodeId: string): ProjectDoc {
  const next = cloneDoc(doc);
  const frame = next.frames.find((f) => f.id === frameId);
  if (frame) frame.nodes = frame.nodes.filter((n) => n.id !== nodeId);
  return next;
}

/** ノードの描画順を1つ前/後ろへ（delta: -1=背面へ, +1=前面へ） */
export function moveNodeLayer(doc: ProjectDoc, frameId: string, nodeId: string, delta: 1 | -1): ProjectDoc {
  const next = cloneDoc(doc);
  const frame = next.frames.find((f) => f.id === frameId);
  if (!frame) return next;
  const idx = frame.nodes.findIndex((n) => n.id === nodeId);
  const target = idx + delta;
  if (idx < 0 || target < 0 || target >= frame.nodes.length) return next;
  const [node] = frame.nodes.splice(idx, 1);
  frame.nodes.splice(target, 0, node);
  return next;
}
