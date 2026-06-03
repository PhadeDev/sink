import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Ms } from "../Icons";

/** Candidate locations for a freedesktop icon name, tried in order. */
function candidatePaths(iconName: string): string[] {
  return [
    `/usr/share/icons/hicolor/64x64/apps/${iconName}.png`,
    `/usr/share/icons/hicolor/48x48/apps/${iconName}.png`,
    `/usr/share/icons/hicolor/scalable/apps/${iconName}.svg`,
    `/usr/share/pixmaps/${iconName}.png`,
    `/usr/share/pixmaps/${iconName}.svg`,
  ];
}

interface AppIconProps {
  iconName: string | null;
}

/**
 * Resolves an app's freedesktop icon by stepping through likely theme paths
 * via the Tauri asset protocol; falls back to a generic glyph.
 */
export function AppIcon({ iconName }: AppIconProps) {
  const [candidateIdx, setCandidateIdx] = useState(0);

  useEffect(() => setCandidateIdx(0), [iconName]);

  const candidates = iconName ? candidatePaths(iconName) : [];
  const src =
    candidateIdx < candidates.length ? convertFileSrc(candidates[candidateIdx]) : null;

  if (src) {
    return <img src={src} alt="" onError={() => setCandidateIdx((i) => i + 1)} />;
  }

  return <Ms name="graphic_eq" />;
}
