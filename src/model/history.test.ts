import { describe, expect, it } from 'vitest';
import { createProject } from './factory';
import {
  canRedo,
  canUndo,
  createHistory,
  pushHistory,
  redoHistory,
  undoHistory,
} from './history';
import type { ProjectDoc } from './types';

function renamed(doc: ProjectDoc, name: string): ProjectDoc {
  return { ...doc, name };
}

describe('履歴 Undo/Redo', () => {
  it('push → undo → redo で文書が往復する', () => {
    const d0 = createProject('A');
    let h = createHistory(d0);
    expect(canUndo(h)).toBe(false);

    h = pushHistory(h, renamed(h.present, 'B'));
    h = pushHistory(h, renamed(h.present, 'C'));
    expect(h.present.name).toBe('C');
    expect(canUndo(h)).toBe(true);

    h = undoHistory(h);
    expect(h.present.name).toBe('B');
    h = undoHistory(h);
    expect(h.present.name).toBe('A');
    expect(canUndo(h)).toBe(false);
    expect(canRedo(h)).toBe(true);

    h = redoHistory(h);
    expect(h.present.name).toBe('B');
    h = redoHistory(h);
    expect(h.present.name).toBe('C');
    expect(canRedo(h)).toBe(false);
  });

  it('undo 後に push すると redo 履歴は消える', () => {
    let h = createHistory(createProject('A'));
    h = pushHistory(h, renamed(h.present, 'B'));
    h = undoHistory(h);
    h = pushHistory(h, renamed(h.present, 'X'));
    expect(canRedo(h)).toBe(false);
    expect(h.present.name).toBe('X');
  });

  it('空の履歴で undo/redo しても壊れない', () => {
    const h = createHistory(createProject('A'));
    expect(undoHistory(h).present.name).toBe('A');
    expect(redoHistory(h).present.name).toBe('A');
  });
});
