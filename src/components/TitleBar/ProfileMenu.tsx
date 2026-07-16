import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";
import { Popover } from "../Popover";

/**
 * Profile picker. Profiles are live-bound: every mixer change autosaves
 * into the active profile, so rows just switch - there is no Save button.
 */
export function ProfileMenu() {
  const [open, setOpen] = useState(false);
  const [newName, setNewName] = useState("");
  /** Profile whose auto-switch (trigger) panel is expanded. */
  const [triggerFor, setTriggerFor] = useState<string | null>(null);
  const profiles = useMixerStore((s) => s.profiles);
  const activeProfile = useMixerStore((s) => s.activeProfile);
  const outputDevices = useMixerStore((s) => s.outputDevices);
  const loadProfile = useMixerStore((s) => s.loadProfile);
  const deleteProfile = useMixerStore((s) => s.deleteProfile);
  const setProfileTrigger = useMixerStore((s) => s.setProfileTrigger);
  const createBlankProfile = useMixerStore((s) => s.createBlankProfile);

  const close = () => {
    setOpen(false);
    setTriggerFor(null);
    setNewName("");
  };

  const create = () => {
    const name = newName.trim();
    if (!name) return;
    void createBlankProfile(name);
    close();
  };

  const triggerLabel = (device: string | null) => {
    if (!device) return null;
    return outputDevices.find((d) => d.name === device)?.description ?? device;
  };

  return (
    <div style={{ position: "relative" }}>
      <button className="select" onClick={() => setOpen((o) => !o)} title="Profiles">
        <Ms name="bookmarks" />
        <span>{activeProfile ?? "Profiles"}</span>
        <Ms name="expand_more" />
      </button>
      <Popover open={open} onClose={close} side="bottom" align="end" style={{ minWidth: 240 }}>
        {profiles.map((profile) => {
          const isActive = profile.name === activeProfile;
          const trigger = triggerLabel(profile.trigger_device);
          return (
            <div key={profile.name}>
              <div
                className={"menu-item profile-row" + (isActive ? " sel" : "")}
                onClick={() => {
                  if (!isActive) void loadProfile(profile.name);
                  close();
                }}
              >
                <Ms name={isActive ? "check" : "bookmark"} />
                <div className="profile-row-main">
                  <span>{profile.name}</span>
                  {trigger && (
                    <span className="profile-row-trigger">
                      <Ms name="bolt" style={{ fontSize: 12 }} />
                      auto-loads with {trigger}
                    </span>
                  )}
                </div>
                <div className="profile-row-actions">
                  <button
                    className="row-icon-btn"
                    title="Auto-load when a device connects"
                    aria-label={`Auto-switch settings for ${profile.name}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      setTriggerFor((t) => (t === profile.name ? null : profile.name));
                    }}
                  >
                    <Ms name="bolt" style={{ fontSize: 15 }} />
                  </button>
                  <button
                    className="row-icon-btn danger"
                    title="Delete profile"
                    aria-label={`Delete profile ${profile.name}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      void deleteProfile(profile.name);
                    }}
                  >
                    <Ms name="delete" style={{ fontSize: 15 }} />
                  </button>
                </div>
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
          );
        })}

        <div className="menu-sep" />
        <div className="menu-save">
          <input
            className="menu-input"
            placeholder="New profile name…"
            value={newName}
            maxLength={64}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") create();
            }}
          />
          <button
            className="select"
            onClick={create}
            disabled={!newName.trim()}
            title="Create a fresh profile (default channels, no routing) and switch to it"
          >
            <Ms name="add" />
            <span>Create</span>
          </button>
        </div>
      </Popover>
    </div>
  );
}
