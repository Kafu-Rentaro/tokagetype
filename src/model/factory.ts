import type {
  Action,
  ButtonNode,
  CameraDef,
  Frame,
  Node,
  NodeType,
  ProjectDoc,
  RectNode,
  TextNode,
  View3DNode,
} from './types';

let counter = 0;

/** 文書内で一意なIDを生成する */
export function newId(prefix = 'id'): string {
  counter += 1;
  const rand =
    typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID().slice(0, 8)
      : Math.random().toString(36).slice(2, 10);
  return `${prefix}_${rand}${counter.toString(36)}`;
}

export function frontCamera(): CameraDef {
  return { id: 'cam_front', name: 'front', position: [0, 0.8, 3.2], target: [0, 0.8, 0] };
}

export function backCamera(): CameraDef {
  return { id: 'cam_back', name: 'back', position: [0, 0.8, -3.2], target: [0, 0.8, 0] };
}

export function defaultCameras(): CameraDef[] {
  return [frontCamera(), backCamera()];
}

export function createFrame(name = 'フレーム'): Frame {
  return {
    id: newId('frame'),
    name,
    width: 390,
    height: 844,
    background: '#ffffff',
    nodes: [],
  };
}

export function createProject(name = '新規プロジェクト'): ProjectDoc {
  const frame = createFrame('ホーム');
  return {
    version: 1,
    id: newId('proj'),
    name,
    frames: [frame],
    assets: [],
    startFrameId: frame.id,
  };
}

export function createNode(type: NodeType, x = 40, y = 40): Node {
  const base = { id: newId('node'), x, y, rotation: 0, opacity: 1, visible: true };
  switch (type) {
    case 'rect': {
      const n: RectNode = {
        ...base, type: 'rect', name: '矩形', width: 160, height: 100,
        fill: '#4f8ff7', cornerRadius: 8,
      };
      return n;
    }
    case 'text': {
      const n: TextNode = {
        ...base, type: 'text', name: 'テキスト', width: 220, height: 40,
        text: 'テキスト', fontSize: 20, fill: '#222222', fontStyle: 'normal', align: 'left',
      };
      return n;
    }
    case 'button': {
      const n: ButtonNode = {
        ...base, type: 'button', name: 'ボタン', width: 180, height: 48,
        label: 'ボタン', fill: '#111111', textColor: '#ffffff', cornerRadius: 24, fontSize: 16,
        actions: [],
      };
      return n;
    }
    case 'view3d': {
      const cameras = defaultCameras();
      const n: View3DNode = {
        ...base, type: 'view3d', name: '3Dビュー', width: 310, height: 380,
        assetId: null, activeCameraId: cameras[0].id, cameras, autoRotate: false,
      };
      return n;
    }
  }
}

export function defaultAction(type: Action['type'], doc: ProjectDoc, frame: Frame): Action {
  switch (type) {
    case 'setCamera': {
      const view = frame.nodes.find((n): n is View3DNode => n.type === 'view3d')
        ?? doc.frames.flatMap((f) => f.nodes).find((n): n is View3DNode => n.type === 'view3d');
      return {
        type: 'setCamera',
        viewNodeId: view?.id ?? '',
        cameraId: view?.cameras[0]?.id ?? '',
      };
    }
    case 'navigate':
      return { type: 'navigate', frameId: doc.frames[0]?.id ?? '' };
    case 'toggleVisible':
      return { type: 'toggleVisible', nodeId: frame.nodes[0]?.id ?? '' };
    case 'openUrl':
      return { type: 'openUrl', url: 'https://example.com' };
  }
}
