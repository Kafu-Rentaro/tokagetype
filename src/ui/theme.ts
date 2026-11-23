// 共有スタイル（SPEC「デザイン方針」: 製図台）。色は CSS 変数経由で styles.css のトークンを参照する。
import type { CSSProperties } from 'react';

export const btn: CSSProperties = {
  display: 'inline-flex', alignItems: 'center', gap: 6,
  background: 'var(--panel-2)', color: 'var(--text)',
  border: '1px solid var(--line)', borderRadius: 'var(--radius)',
  padding: '5px 10px', cursor: 'pointer', fontSize: 13, whiteSpace: 'nowrap',
};

export const btnActive: CSSProperties = {
  background: 'var(--accent)', borderColor: 'var(--accent)', color: 'var(--accent-ink)',
};

export const miniBtn: CSSProperties = {
  display: 'inline-flex', alignItems: 'center', gap: 4,
  background: 'var(--panel-2)', color: 'var(--text)',
  border: '1px solid var(--line)', borderRadius: 'var(--radius)',
  padding: '3px 8px', cursor: 'pointer', fontSize: 11, whiteSpace: 'nowrap',
};

export const miniBtnActive: CSSProperties = {
  background: 'var(--accent)', borderColor: 'var(--accent)', color: 'var(--accent-ink)',
};

export const iconBtn: CSSProperties = {
  display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
  background: 'transparent', color: 'var(--text-dim)', border: 'none',
  cursor: 'pointer', padding: 4, borderRadius: 'var(--radius)',
};

export const input: CSSProperties = {
  width: '100%', background: 'var(--inset)', color: 'var(--text)',
  border: '1px solid var(--line)', borderRadius: 'var(--radius)',
  padding: '5px 7px', fontSize: 13, boxSizing: 'border-box',
};

export const numInput: CSSProperties = { ...input, fontFamily: 'var(--mono)' };

export const select: CSSProperties = {
  background: 'var(--inset)', color: 'var(--text)',
  border: '1px solid var(--line)', borderRadius: 'var(--radius)',
  padding: '5px 8px', fontSize: 13,
};

export const panelLeft: CSSProperties = {
  width: 232, background: 'var(--panel)', color: 'var(--text)',
  overflowY: 'auto', borderRight: '1px solid var(--line)', fontSize: 13, flexShrink: 0,
};

export const panelRight: CSSProperties = {
  width: 288, background: 'var(--panel)', color: 'var(--text)',
  overflowY: 'auto', borderLeft: '1px solid var(--line)', fontSize: 13, flexShrink: 0,
};

export const group: CSSProperties = {
  padding: '12px 12px 14px', borderBottom: '1px solid var(--line)',
};

export const groupTitle: CSSProperties = {
  fontWeight: 700, color: 'var(--text)', marginBottom: 8, fontSize: 11,
  letterSpacing: '0.08em',
};

export const field: CSSProperties = { display: 'block', marginBottom: 8 };

export const fieldLabel: CSSProperties = {
  display: 'block', fontSize: 11, color: 'var(--text-dim)', marginBottom: 3,
};

export const divider: CSSProperties = {
  width: 1, height: 22, background: 'var(--line)', margin: '0 2px', flexShrink: 0,
};
