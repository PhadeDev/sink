import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";
import { Popover } from "../Popover";

/**
 * Headerbar profile picker: load/delete saved profiles, save the current
 * mixer state under a new name, and bind a trigger device that auto-loads
 * a profile when it appears (Phase 5).
 */
export function ProfileMenu() {
  const [open, setOpen] = useState(false);
  const [newName, setNewName] = useState("");
  /** Profile whose trigger submenu is expanded. */
  const [triggerFor, setTriggerFor] = useState<string | null>(null);
  const profiles = useMixerStore((s) => s.profiles);
  const activeProfile = useMixerStore((s) => s.activeProfile);
  const outputDevices = useMixerStore((s) => s.outputDevices);
  const saveProfile = useMixerStore((s) => s.saveProfile);
  const loadProfile = useMixerStore((s) => s.loadProfile);
  const deleteProfile = useMixerStore((s) => s.deleteProfile);
  const setProfileTrigger = useMixerStore((s) => s.setProfileTrigger);
  const createBlankProfile = useMixerStore((s) => s.createBlankProfile);

  const save = () => {
    const name = newName.trim();
    if (!name) return;
    void saveProfile(name);
    setNewName("");
    setOpen(false);
  };

  const createBlank = () => {
    const name = newName.trim();
    if (!name) return;
    void createBlankProfile(name);
    setNewName("");
  };

  const close = () => {
    setOpen(false);
    setTriggerFor(null);
  };

  return (
    <div style={{ position: "relative" }}>
      <button className="select" onClick={() => setOpen((o) => !o)} title="Profiles">
        <Ms name="bookmarks" />
        <span>{activeProfile ?? "Profiles"}</span>
        <Ms name="expand_more" />
      </button>
      <Popover open={open} onClose={close} style={{ top: 38, right: 0, minWidth: 280 }}>
        {profiles.length === 0 && (
          <div className="menu-item" style={{ cursor: "default", color: "var(--fg-muted)" }}>
            No saved profiles
          </div>
        )}
        {profiles.map((profile) => (
          <div key={profile.name}>
            <div
              className={"menu-item" + (profile.name === activeProfile ? " sel" : "")}
              onClick={() => {
                void loadProfile(profile.name);
                close();
              }}
            >
              <Ms name="bookmark" />
              <span style={{ flex: 1 }}>{profile.name}</span>
              {profile.trigger_device && (
                <Ms
                  name="bolt"
                  style={{ fontSize: 14, color: "var(--warning)" }}
                />
              )}
              <button
                className="wbtn"
                style={{ width: 22, height: 22 }}
                aria-label={`Auto-switch settings for ${profile.name}`}
                title="Auto-load when a device connects"
                onClick={(e) => {
                  e.stopPropagation();
                  setTriggerFor((t) => (t === profile.name ? null : profile.name));
                }}
              >
                <Ms name="bolt" style={{ fontSize: 14 }} />
              </button>
              <button
                className="wbtn"
                style={{ width: 22, height: 22 }}
                aria-label={`Delete profile ${profile.name}`}
                onClick={(e) => {
                  e.stopPropagation();
                  void deleteProfile(profile.name);
                }}
              >
                <Ms name="delete" style={{ fontSize: 14 }} />
              </button>
            </div>
            {triggerFor === profile.name && (
              <div className="trigger-panel">
                <div className="trigger-hint">Auto-load when this device connects:</div>
                <div
                  className={"menu-item" + (profile.trigger_device === null ? " sel" : "")}
                  onClick={() => {
                    void setProfileTrigger(profile.name, null);
                    setTriggerFor(null);
                  }}
                >
                  <Ms name="block" />
                  <span>No auto-switch</span>
                </div>
                {outputDevices.map((d) => (
                  <div
                    key={d.name}
                    className={
                      "menu-item" + (d.name === profile.trigger_device ? " sel" : "")
                    }
                    onClick={() => {
                      void setProfileTrigger(profile.name, d.name);
                      setTriggerFor(null);
                    }}
                  >
                    <Ms name="speaker" />
                    <span>{d.description}</span>
                  </div>
                ))}
              </div>
            )}
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
          <button
            className="select"
            onClick={save}
            disabled={!newName.trim()}
            title="Save the current mixer state under this name"
          >
            <Ms name="save" />
            <span>Save</span>
          </button>
          <button
            className="select"
            onClick={createBlank}
            disabled={!newName.trim()}
            title="Create a clean-slate profile (default channels, no routing)"
          >
            <Ms name="note_add" />
            <span>Blank</span>
          </button>
        </div>
      </Popover>
    </div>
  );
}
