import type { SVGProps } from 'react';

/** Small stroke icon set drawn for GamepadLab (24×24 grid, currentColor). */
const paths = {
  gamepad:
    'M6 9h4M8 7v4M15 10h.01M18 8h.01M7.5 4h9a5.5 5.5 0 0 1 5.4 6.5l-1.1 6.2a2.6 2.6 0 0 1-4.6 1.1L14 15h-4l-2.2 2.8a2.6 2.6 0 0 1-4.6-1.1L2.1 10.5A5.5 5.5 0 0 1 7.5 4z',
  usb: 'M12 3v14M12 21a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM12 7l-3 3v2M12 9l3-2V5M9 12H7v2h4M15 5h2V3h-2z',
  bluetooth: 'M7 7l10 10-5 4V3l5 4L7 17',
  battery: 'M3 8h15v8H3zM21 11v2',
  settings:
    'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z',
  info: 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 16v-4M12 8h.01',
  panel: 'M3 4h18v16H3zM15 4v16',
  play: 'M7 4l13 8-13 8z',
  stop: 'M6 6h12v12H6z',
  check: 'M4 12l5 5L20 6',
  x: 'M6 6l12 12M18 6L6 18',
  alert: 'M12 3l10 18H2zM12 10v5M12 18h.01',
  download: 'M12 3v12M7 10l5 5 5-5M4 19h16',
  upload: 'M12 21V9M7 14l5-5 5 5M4 5h16',
  copy: 'M9 9h11v11H9zM5 15H4V4h11v1',
  trash: 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3',
  plus: 'M12 5v14M5 12h14',
  refresh: 'M20 11a8 8 0 0 0-14.9-3M4 4v4h4M4 13a8 8 0 0 0 14.9 3M20 20v-4h-4',
  activity: 'M3 12h4l3-8 4 16 3-8h4',
  target: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM12 12h.01',
  compass: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM15.5 8.5l-2 5-5 2 2-5z',
  history: 'M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5M12 7v5l3 2',
  cast: 'M2 16a5 5 0 0 1 5 5M2 12a9 9 0 0 1 9 9M2 8V5a1 1 0 0 1 1-1h18a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-7',
  sliders: 'M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6',
  vibrate: 'M8 4h8v16H8zM4 8v8M20 8v8M2 10v4M22 10v4',
  layers: 'M12 2l10 6-10 6L2 8zM2 14l10 6 10-6',
  sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10zM12 1v2M12 21v2M4.2 4.2l1.4 1.4M18.4 18.4l1.4 1.4M1 12h2M21 12h2M4.2 19.8l1.4-1.4M18.4 5.6l1.4-1.4',
  moon: 'M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z',
  keyboard: 'M2 6h20v12H2zM6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10',
  shield: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z',
  chevron: 'M9 6l6 6-6 6',
  file: 'M14 3H6v18h12V7zM14 3v4h4',
  touch:
    'M9 11V5a2 2 0 1 1 4 0v6M13 10a2 2 0 1 1 4 0v2M17 11a2 2 0 1 1 4 0v4a7 7 0 0 1-7 7h-1a7 7 0 0 1-6-3.4L4 15a2 2 0 0 1 3.3-2.2L9 15',
  bolt: 'M13 2L4 14h7l-1 8 9-12h-7z',
} as const;

export type IconName = keyof typeof paths;

export function Icon({
  name,
  size = 18,
  ...rest
}: { name: IconName; size?: number } & SVGProps<SVGSVGElement>) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      <path d={paths[name]} />
    </svg>
  );
}
