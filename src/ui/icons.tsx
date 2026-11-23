// 線画 SVG アイコン（stroke: currentColor）。絵文字ラベルの代替（SPEC「デザイン方針」）。
import type { CSSProperties } from 'react';

interface IconProps {
  size?: number;
  style?: CSSProperties;
}

function base(size: number) {
  return {
    width: size,
    height: size,
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.8,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    'aria-hidden': true,
    focusable: false,
  };
}

function Svg({ size = 16, style, children }: IconProps & { children: React.ReactNode }) {
  return (
    <svg {...base(size)} style={{ flexShrink: 0, ...style }}>
      {children}
    </svg>
  );
}

export const IconPlus = (p: IconProps) => (
  <Svg {...p}><path d="M12 5v14M5 12h14" /></Svg>
);
export const IconTrash = (p: IconProps) => (
  <Svg {...p}><path d="M4 7h16M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2m3 0v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V7" /><path d="M10 11v6M14 11v6" /></Svg>
);
export const IconDownload = (p: IconProps) => (
  <Svg {...p}><path d="M12 4v11m0 0 4-4m-4 4-4-4" /><path d="M4 19h16" /></Svg>
);
export const IconUpload = (p: IconProps) => (
  <Svg {...p}><path d="M12 15V4m0 0 4 4m-4-4L8 8" /><path d="M4 19h16" /></Svg>
);
export const IconUndo = (p: IconProps) => (
  <Svg {...p}><path d="M8 5 4 9l4 4" /><path d="M4 9h9a7 7 0 0 1 0 14h-3" /></Svg>
);
export const IconRedo = (p: IconProps) => (
  <Svg {...p}><path d="m16 5 4 4-4 4" /><path d="M20 9h-9a7 7 0 0 0 0 14h3" /></Svg>
);
export const IconPlay = (p: IconProps) => (
  <Svg {...p}><path d="M7 4.8v14.4a1 1 0 0 0 1.5.87l11-7.2a1 1 0 0 0 0-1.74l-11-7.2A1 1 0 0 0 7 4.8Z" /></Svg>
);
export const IconPencil = (p: IconProps) => (
  <Svg {...p}><path d="M16.5 4.5a2.1 2.1 0 0 1 3 3L8 19l-4.2 1.2L5 16Z" /><path d="m14.5 6.5 3 3" /></Svg>
);
export const IconRect = (p: IconProps) => (
  <Svg {...p}><rect x="4" y="6" width="16" height="12" rx="1" /></Svg>
);
export const IconType = (p: IconProps) => (
  <Svg {...p}><path d="M5 5h14M12 5v14" /></Svg>
);
export const IconButtonShape = (p: IconProps) => (
  <Svg {...p}><rect x="3" y="8" width="18" height="8" rx="4" /><path d="M9 12h6" /></Svg>
);
export const IconCube = (p: IconProps) => (
  <Svg {...p}><path d="M12 3 4 7.5v9L12 21l8-4.5v-9Z" /><path d="M4 7.5 12 12l8-4.5M12 12v9" /></Svg>
);
export const IconEye = (p: IconProps) => (
  <Svg {...p}><path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z" /><circle cx="12" cy="12" r="3" /></Svg>
);
export const IconEyeOff = (p: IconProps) => (
  <Svg {...p}><path d="M4 4l16 16" /><path d="M10.6 6c.45-.06.92-.1 1.4-.1 6 0 9.5 6.1 9.5 6.1a17 17 0 0 1-3 3.5M6 8A16.6 16.6 0 0 0 2.5 12S6 18.1 12 18.1c1.2 0 2.3-.24 3.3-.64" /><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" /></Svg>
);
export const IconChevronUp = (p: IconProps) => (
  <Svg {...p}><path d="m5 15 7-7 7 7" /></Svg>
);
export const IconChevronDown = (p: IconProps) => (
  <Svg {...p}><path d="m5 9 7 7 7-7" /></Svg>
);
export const IconX = (p: IconProps) => (
  <Svg {...p}><path d="M6 6l12 12M18 6 6 18" /></Svg>
);
export const IconMonitor = (p: IconProps) => (
  <Svg {...p}><rect x="3" y="4" width="18" height="12" rx="1" /><path d="M9 20h6M12 16v4" /></Svg>
);
export const IconSmartphone = (p: IconProps) => (
  <Svg {...p}><rect x="7" y="3" width="10" height="18" rx="2" /><path d="M11 18h2" /></Svg>
);
export const IconLayers = (p: IconProps) => (
  <Svg {...p}><path d="m12 3 9 5-9 5-9-5Z" /><path d="m3 13 9 5 9-5" /></Svg>
);
export const IconSliders = (p: IconProps) => (
  <Svg {...p}><path d="M5 7h9M18 7h1M5 17h3M12 17h7" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></Svg>
);
export const IconWarn = (p: IconProps) => (
  <Svg {...p}><path d="M12 4 2.8 20h18.4Z" /><path d="M12 10v4.5" /><circle cx="12" cy="17.2" r="0.4" fill="currentColor" /></Svg>
);
export const IconCopy = (p: IconProps) => (
  <Svg {...p}><rect x="9" y="9" width="11" height="11" rx="1" /><path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" /></Svg>
);
export const IconBack = (p: IconProps) => (
  <Svg {...p}><path d="M9 6 4 11l5 5" /><path d="M4 11h10a6 6 0 0 1 0 12h-3" /></Svg>
);
