import { useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { createPortal } from "react-dom";

interface PopoverProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  /** Which side of the anchor to open on. */
  side?: "top" | "bottom";
  /** Horizontal alignment relative to the anchor. */
  align?: "start" | "center" | "end";
  /** Extra styles (e.g. minWidth) merged onto the menu. */
  style?: CSSProperties;
}

const MARGIN = 8;
const GAP = 6;

/**
 * Anchored popover menu, rendered through a portal so it can never be
 * clipped by scroll containers or stack under the nav rail. The anchor is
 * the parent element of the marker span (call sites wrap trigger+Popover
 * in a relative container, which keeps working unchanged).
 */
export function Popover({ open, onClose, children, side = "bottom", align = "start", style }: PopoverProps) {
  const markerRef = useRef<HTMLSpanElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<CSSProperties | null>(null);

  useLayoutEffect(() => {
    if (!open) {
      setPosition(null);
      return;
    }
    const anchor = markerRef.current?.parentElement;
    const menu = menuRef.current;
    if (!anchor || !menu) return;

    const rect = anchor.getBoundingClientRect();
    const menuRect = menu.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    let left: number;
    if (align === "center") {
      left = rect.left + rect.width / 2 - menuRect.width / 2;
    } else if (align === "end") {
      left = rect.right - menuRect.width;
    } else {
      left = rect.left;
    }
    left = Math.max(MARGIN, Math.min(left, vw - menuRect.width - MARGIN));

    let top: number;
    if (side === "top") {
      top = rect.top - menuRect.height - GAP;
      if (top < MARGIN) top = rect.bottom + GAP; // flip when cramped
    } else {
      top = rect.bottom + GAP;
      if (top + menuRect.height > vh - MARGIN) top = rect.top - menuRect.height - GAP;
    }
    top = Math.max(MARGIN, Math.min(top, vh - menuRect.height - MARGIN));

    setPosition({ left, top });
  }, [open, side, align]);

  return (
    <>
      <span ref={markerRef} style={{ display: "none" }} aria-hidden="true" />
      {open &&
        createPortal(
          <>
            <div className="scrim" onClick={onClose} />
            <div
              ref={menuRef}
              className="menu"
              style={{
                position: "fixed",
                visibility: position ? "visible" : "hidden",
                ...(position ?? { left: 0, top: 0 }),
                ...style,
              }}
            >
              {children}
            </div>
          </>,
          document.body,
        )}
    </>
  );
}
