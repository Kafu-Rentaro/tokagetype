import type { ProjectDoc } from './types';

/** 文書スナップショットによる Undo/Redo 履歴（純粋関数群） */
export interface History {
  past: ProjectDoc[];
  present: ProjectDoc;
  future: ProjectDoc[];
}

export const HISTORY_LIMIT = 100;

export function createHistory(doc: ProjectDoc): History {
  return { past: [], present: doc, future: [] };
}

export function pushHistory(h: History, next: ProjectDoc): History {
  if (next === h.present) return h;
  const past = [...h.past, h.present];
  if (past.length > HISTORY_LIMIT) past.splice(0, past.length - HISTORY_LIMIT);
  return { past, present: next, future: [] };
}

export function undoHistory(h: History): History {
  if (h.past.length === 0) return h;
  const previous = h.past[h.past.length - 1];
  return {
    past: h.past.slice(0, -1),
    present: previous,
    future: [h.present, ...h.future],
  };
}

export function redoHistory(h: History): History {
  if (h.future.length === 0) return h;
  const [next, ...rest] = h.future;
  return {
    past: [...h.past, h.present],
    present: next,
    future: rest,
  };
}

export function canUndo(h: History): boolean {
  return h.past.length > 0;
}

export function canRedo(h: History): boolean {
  return h.future.length > 0;
}
