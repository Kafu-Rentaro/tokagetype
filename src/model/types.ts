// 文書モデル（docs/SPEC.md「文書モデル」準拠）

export interface ProjectDoc {
  version: 1;
  id: string;
  name: string;
  frames: Frame[];
  assets: AssetRef[];
  startFrameId: string;
}

export interface Frame {
  id: string;
  name: string;
  width: number;
  height: number;
  background: string;
  nodes: Node[];
}

export type Node = RectNode | TextNode | ButtonNode | View3DNode;

export interface NodeBase {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation?: number;
  opacity?: number;
  visible?: boolean;
  actions?: Action[];
}

export interface RectNode extends NodeBase {
  type: 'rect';
  fill: string;
  cornerRadius: number;
  stroke?: string;
  strokeWidth?: number;
}

export interface TextNode extends NodeBase {
  type: 'text';
  text: string;
  fontSize: number;
  fill: string;
  fontStyle?: string;
  align?: 'left' | 'center' | 'right';
}

export interface ButtonNode extends NodeBase {
  type: 'button';
  label: string;
  fill: string;
  textColor: string;
  cornerRadius: number;
  fontSize: number;
}

export interface View3DNode extends NodeBase {
  type: 'view3d';
  assetId: string | null;
  activeCameraId: string;
  cameras: CameraDef[];
  autoRotate?: boolean;
}

export interface CameraDef {
  id: string;
  name: string;
  position: [number, number, number];
  target: [number, number, number];
}

export interface AssetRef {
  id: string;
  name: string;
  kind: 'glb';
}

export type Action =
  | { type: 'setCamera'; viewNodeId: string; cameraId: string }
  | { type: 'navigate'; frameId: string }
  | { type: 'toggleVisible'; nodeId: string }
  | { type: 'openUrl'; url: string };

export type NodeType = Node['type'];
