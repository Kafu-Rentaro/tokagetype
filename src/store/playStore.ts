import { create } from 'zustand';
import type { Action, ProjectDoc } from '../model/types';

/**
 * 再生モードのランタイム状態。文書には保存しない（SPEC「再生モード」）。
 * - frameId: 表示中フレーム
 * - cameraOverrides: view3d ノードID → アクティブカメラID（setCamera の結果）
 * - visibilityOverrides: ノードID → 表示状態（toggleVisible の結果）
 * - cameraNonce: 同じカメラへの再切替でもアニメーションを再生するためのカウンタ
 */
interface PlayState {
  frameId: string | null;
  cameraOverrides: Record<string, string>;
  visibilityOverrides: Record<string, boolean>;
  cameraNonce: Record<string, number>;

  start: (doc: ProjectDoc) => void;
  stop: () => void;
  runAction: (doc: ProjectDoc, action: Action) => void;
  runActions: (doc: ProjectDoc, actions: Action[] | undefined) => void;
  backToStart: (doc: ProjectDoc) => void;
  activeCameraId: (nodeId: string, fallback: string) => string;
  isVisible: (nodeId: string, fallback: boolean) => boolean;
}

export const usePlayStore = create<PlayState>((set, get) => ({
  frameId: null,
  cameraOverrides: {},
  visibilityOverrides: {},
  cameraNonce: {},

  start: (doc) =>
    set({
      frameId: doc.startFrameId,
      cameraOverrides: {},
      visibilityOverrides: {},
      cameraNonce: {},
    }),

  stop: () => set({ frameId: null, cameraOverrides: {}, visibilityOverrides: {}, cameraNonce: {} }),

  runAction: (doc, action) => {
    switch (action.type) {
      case 'setCamera': {
        // 参照先が存在しない場合は無視（SPEC）
        let exists = false;
        for (const f of doc.frames) {
          const n = f.nodes.find((node) => node.id === action.viewNodeId);
          if (n && n.type === 'view3d' && n.cameras.some((c) => c.id === action.cameraId)) exists = true;
        }
        if (!exists) return;
        set((s) => ({
          cameraOverrides: { ...s.cameraOverrides, [action.viewNodeId]: action.cameraId },
          cameraNonce: { ...s.cameraNonce, [action.viewNodeId]: (s.cameraNonce[action.viewNodeId] ?? 0) + 1 },
        }));
        break;
      }
      case 'navigate':
        if (doc.frames.some((f) => f.id === action.frameId)) set({ frameId: action.frameId });
        break;
      case 'toggleVisible': {
        let base: boolean | undefined;
        for (const f of doc.frames) {
          const n = f.nodes.find((node) => node.id === action.nodeId);
          if (n) base = n.visible !== false;
        }
        if (base === undefined) return;
        set((s) => {
          const current = s.visibilityOverrides[action.nodeId] ?? base;
          return { visibilityOverrides: { ...s.visibilityOverrides, [action.nodeId]: !current } };
        });
        break;
      }
      case 'openUrl':
        if (/^https?:\/\//.test(action.url)) window.open(action.url, '_blank', 'noopener');
        break;
    }
  },

  runActions: (doc, actions) => {
    for (const action of actions ?? []) get().runAction(doc, action);
  },

  backToStart: (doc) =>
    set({ frameId: doc.startFrameId, cameraOverrides: {}, visibilityOverrides: {}, cameraNonce: {} }),

  activeCameraId: (nodeId, fallback) => get().cameraOverrides[nodeId] ?? fallback,
  isVisible: (nodeId, fallback) => get().visibilityOverrides[nodeId] ?? fallback,
}));
