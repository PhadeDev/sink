import type { CSSProperties } from "react";

/** Material Symbol glyph (self-hosted via the material-symbols package). */
export function Ms({
  name,
  className,
  style,
}: {
  name: string;
  className?: string;
  style?: CSSProperties;
}) {
  return (
    <span
      className={"ms material-symbols-outlined" + (className ? " " + className : "")}
      style={style}
      aria-hidden="true"
    >
      {name}
    </span>
  );
}

/** App logo mark — stylized sink/funnel of soundwaves. */
export function SinkMark() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round">
      <path d="M3 5 L21 5 L13.5 13 L13.5 20 L10.5 20 L10.5 13 Z" fill="currentColor" stroke="none" opacity="0.92" />
    </svg>
  );
}

/** Material Symbol name for each virtual channel. */
export const CHANNEL_ICONS: Record<string, string> = {
  sink_game: "sports_esports",
  sink_chat: "forum",
  sink_music: "music_note",
  sink_system: "desktop_windows",
};

export function channelIcon(sinkName: string): string {
  return CHANNEL_ICONS[sinkName] ?? "graphic_eq";
}
