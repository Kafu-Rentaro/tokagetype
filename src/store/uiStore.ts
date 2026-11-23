import { create } from 'zustand';

export type UiMode = 'pc' | 'mobile';
export type SheetKind = 'left' | 'right' | null;

const UI_MODE_KEY = 'tokagetype:uiMode';
export const MOBILE_BREAKPOINT = 768;

function autoUiMode(): UiMode {
  return typeof window !== 'undefined' && window.innerWidth < MOBILE_BREAKPOINT ? 'mobile' : 'pc';
}

function initialUiMode(): UiMode {
  try {
    const saved = localStorage.getItem(UI_MODE_KEY);
    if (saved === 'pc' || saved === 'mobile') return saved;
  } catch {
    /* localStorage 不可の環境では自動判定のみ */
  }
  return autoUiMode();
}

interface UiState {
  uiMode: UiMode;
  /** 手動で選んだことがあるか（ある場合は幅による自動切替をしない） */
  manual: boolean;
  /** スマホ UI で開いているボトムシート */
  sheet: SheetKind;
  setUiMode: (mode: UiMode) => void;
  /** 幅変化時の自動判定。手動選択がある場合は何もしない */
  syncAuto: () => void;
  setSheet: (sheet: SheetKind) => void;
}

export const useUiStore = create<UiState>((set, get) => ({
  uiMode: initialUiMode(),
  manual: (() => {
    try {
      const saved = localStorage.getItem(UI_MODE_KEY);
      return saved === 'pc' || saved === 'mobile';
    } catch {
      return false;
    }
  })(),
  sheet: null,
  setUiMode: (mode) => {
    try {
      localStorage.setItem(UI_MODE_KEY, mode);
    } catch {
      /* 保存できなくても切替自体は行う */
    }
    set({ uiMode: mode, manual: true, sheet: null });
  },
  syncAuto: () => {
    if (get().manual) return;
    const next = autoUiMode();
    if (next !== get().uiMode) set({ uiMode: next, sheet: null });
  },
  setSheet: (sheet) => set({ sheet }),
}));
