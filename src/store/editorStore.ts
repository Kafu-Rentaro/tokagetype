import { create } from 'zustand';
import {
  canRedo as historyCanRedo,
  canUndo as historyCanUndo,
  createHistory,
  pushHistory,
  redoHistory,
  undoHistory,
  type History,
} from '../model/history';
import { cloneDoc, updateFrame, updateNode } from '../model/mutate';
import type { Frame, Node, ProjectDoc } from '../model/types';

export type AppMode = 'edit' | 'play';
export type SaveState = 'saved' | 'dirty' | 'saving';

interface EditorState {
  doc: ProjectDoc | null;
  history: History | null;
  currentFrameId: string | null;
  selectedNodeId: string | null;
  mode: AppMode;
  saveState: SaveState;
  /** 3D操作（OrbitControls）を有効にする view3d ノードID */
  orbitNodeId: string | null;
  /** スカルプト対象の view3d ノードID（モーダル表示中） */
  sculptNodeId: string | null;

  loadDoc: (doc: ProjectDoc) => void;
  /** 履歴に積む文書更新 */
  commit: (next: ProjectDoc) => void;
  /** 関数型の文書更新（履歴に積む） */
  mutate: (fn: (doc: ProjectDoc) => ProjectDoc) => void;
  undo: () => void;
  redo: () => void;
  setCurrentFrame: (frameId: string) => void;
  selectNode: (nodeId: string | null) => void;
  setMode: (mode: AppMode) => void;
  setSaveState: (s: SaveState) => void;
  setOrbitNode: (nodeId: string | null) => void;
  setSculptNode: (nodeId: string | null) => void;

  currentFrame: () => Frame | null;
  selectedNode: () => Node | null;
  patchNode: (nodeId: string, patch: Partial<Node>) => void;
  patchFrame: (frameId: string, patch: Partial<Frame>) => void;
}

export const useEditorStore = create<EditorState>((set, get) => ({
  doc: null,
  history: null,
  currentFrameId: null,
  selectedNodeId: null,
  mode: 'edit',
  saveState: 'saved',
  orbitNodeId: null,
  sculptNodeId: null,

  loadDoc: (doc) =>
    set({
      doc,
      history: createHistory(doc),
      currentFrameId: doc.startFrameId,
      selectedNodeId: null,
      orbitNodeId: null,
      saveState: 'saved',
    }),

  commit: (next) => {
    const { history } = get();
    if (!history) return;
    set({ history: pushHistory(history, next), doc: next, saveState: 'dirty' });
  },

  mutate: (fn) => {
    const { doc } = get();
    if (!doc) return;
    get().commit(fn(cloneDoc(doc)));
  },

  undo: () => {
    const { history } = get();
    if (!history || !historyCanUndo(history)) return;
    const h = undoHistory(history);
    set({ history: h, doc: h.present, saveState: 'dirty', selectedNodeId: null });
  },

  redo: () => {
    const { history } = get();
    if (!history || !historyCanRedo(history)) return;
    const h = redoHistory(history);
    set({ history: h, doc: h.present, saveState: 'dirty', selectedNodeId: null });
  },

  setCurrentFrame: (frameId) => set({ currentFrameId: frameId, selectedNodeId: null, orbitNodeId: null }),
  selectNode: (nodeId) => set({ selectedNodeId: nodeId }),
  setMode: (mode) => set({ mode, orbitNodeId: null }),
  setSaveState: (saveState) => set({ saveState }),
  setOrbitNode: (orbitNodeId) => set({ orbitNodeId }),
  setSculptNode: (sculptNodeId) => set({ sculptNodeId }),

  currentFrame: () => {
    const { doc, currentFrameId } = get();
    if (!doc) return null;
    return doc.frames.find((f) => f.id === currentFrameId) ?? doc.frames[0] ?? null;
  },

  selectedNode: () => {
    const { doc, selectedNodeId } = get();
    if (!doc || !selectedNodeId) return null;
    for (const f of doc.frames) {
      const n = f.nodes.find((node) => node.id === selectedNodeId);
      if (n) return n;
    }
    return null;
  },

  patchNode: (nodeId, patch) => {
    const { doc, currentFrameId } = get();
    if (!doc || !currentFrameId) return;
    get().commit(updateNode(doc, currentFrameId, nodeId, patch as Partial<Node> & Record<string, unknown>));
  },

  patchFrame: (frameId, patch) => {
    const { doc } = get();
    if (!doc) return;
    get().commit(updateFrame(doc, frameId, patch));
  },
}));

export { historyCanRedo as canRedoHistory, historyCanUndo as canUndoHistory };
export type { History };
