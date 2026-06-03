import { useEffect, useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Ms } from "../Icons";

const ICON_BASES = [
  "/usr/share/icons/hicolor",
  "/var/lib/flatpak/exports/share/icons/hicolor",
];
const ICON_SIZES = ["64x64", "128x128", "256x256", "48x48", "32x32"];

/** Candidate file paths for a freedesktop icon name, tried in order. */
function candidatePaths(iconName: string): string[] {
  const paths: string[] = [];
  for (const base of ICON_BASES) {
    for (const size of ICON_SIZES) {
      paths.push(`${base}/${size}/apps/${iconName}.png`);
    }
    paths.push(`${base}/scalable/apps/${iconName}.svg`);
  }
  paths.push(`/usr/share/pixmaps/${iconName}.png`);
  paths.push(`/usr/share/pixmaps/${iconName}.svg`);
  return paths;
}

interface AppIconProps {
  iconName: string | null;
  /** Discovered app name — used to guess an icon when icon_name is absent. */
  appName: string;
}

/**
 * Resolves an app's freedesktop icon by stepping through likely theme and
 * Flatpak-export paths via the Tauri asset protocol. When the stream
 * carries no icon_name (common for apps that only set generic node props),
 * the lowercased app name is tried as the icon name. Falls back to a
 * generic glyph.
 */
export function AppIcon({ iconName, appName }: AppIconProps) {
  const candidates = useMemo(() => {
    const names = new Set<string>();
    if (iconName) names.add(iconName);
    const guess = appName.toLowerCase().replace(/\s+/g, "-");
    if (guess && guess !== "unknown") {
      names.add(guess);
      names.add(guess.replace(/-/g, ""));
    }
    return [...names].flatMap(candidatePaths);
  }, [iconName, appName]);

  const [idx, setIdx] = useState(0);
  useEffect(() => setIdx(0), [candidates]);

  const src = idx < candidates.length ? convertFileSrc(candidates[idx]) : null;

  if (src) {
    return <img src={src} alt="" onError={() => setIdx((i) => i + 1)} />;
  }

  return <Ms name="graphic_eq" />;
}
