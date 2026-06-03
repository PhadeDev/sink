import type { CSSProperties, ReactNode } from "react";

/** Anchored popover menu with a click-away scrim. */
export function Popover({
  open,
  onClose,
  children,
  style,
}: {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  style?: CSSProperties;
}) {
  if (!open) return null;
  return (
    <>
      <div className="scrim" onClick={onClose} />
      <div className="menu" style={style}>
        {children}
      </div>
    </>
  );
}
