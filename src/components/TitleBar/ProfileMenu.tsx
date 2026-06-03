import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";
import { Popover } from "../Popover";

/**
 * Headerbar profile picker: load/delete saved profiles, save the current
 * mixer state under a new name.
 */
export function ProfileMenu() {
  const [open, setOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const profiles = useMixerStore((s) => s.profiles);
  const activeProfile = useMixerStore((s) => s.activeProfile);
  const saveProfile = useMixerStore((s) => s.saveProfile);
  const loadProfile = useMixerStore((s) => s.loadProfile);
  const deleteProfile = useMixerStore((s) => s.deleteProfile);

  const save = () => {
    const name = newName.trim();
    if (!name) return;
    void saveProfile(name);
    setNewName("");
    setOpen(false);
  };

  return (
    <div style={{ position: "relative" }}>
      <button className="select" onClick={() => setOpen((o) => !o)} title="Profiles">
        <Ms name="bookmarks" />
        <span>{activeProfile ?? "Profiles"}</span>
        <Ms name="expand_more" />
      </button>
      <Popover open={open} onClose={() => setOpen(false)} style={{ top: 38, right: 0 }}>
        {profiles.length === 0 && (
          <div className="menu-item" style={{ cursor: "default", color: "var(--fg-muted)" }}>
            No saved profiles
          </div>
        )}
        {profiles.map((name) => (
          <div
            key={name}
            className={"menu-item" + (name === activeProfile ? " sel" : "")}
            onClick={() => {
              void loadProfile(name);
              setOpen(false);
            }}
          >
            <Ms name="bookmark" />
            <span style={{ flex: 1 }}>{name}</span>
            <button
              className="wbtn"
              style={{ width: 22, height: 22 }}
              aria-label={`Delete profile ${name}`}
              onClick={(e) => {
                e.stopPropagation();
                void deleteProfile(name);
              }}
            >
              <Ms name="delete" style={{ fontSize: 14 }} />
            </button>
          </div>
        ))}
        <div className="menu-sep" />
        <div className="menu-save">
          <input
            className="menu-input"
            placeholder="New profile name…"
            value={newName}
            maxLength={64}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") save();
            }}
          />
          <button className="select" onClick={save} disabled={!newName.trim()}>
            <Ms name="save" />
            <span>Save</span>
          </button>
        </div>
      </Popover>
    </div>
  );
}
