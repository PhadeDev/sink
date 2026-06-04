import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useMixerStore } from "../../store/mixer";
import type { OutputDevice } from "../../types";
import { Ms } from "../Icons";
import { Popover } from "../Popover";
import { Toggle } from "../Toggle";

interface DefaultDevices {
  output: string | null;
  input: string | null;
}

/** Card row with a device dropdown for picking a system default. */
function DeviceRow({
  icon,
  title,
  devices,
  current,
  onPick,
}: {
  icon: string;
  title: string;
  devices: OutputDevice[];
  current: string | null;
  onPick: (name: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const currentDesc = devices.find((d) => d.name === current)?.description ?? current ?? "—";

  return (
    <div className="row">
      <div className="ricon">
        <Ms name={icon} />
      </div>
      <div className="rmain">
        <div className="rtitle">{title}</div>
        <div className="rsub">{currentDesc}</div>
      </div>
      <div style={{ position: "relative" }}>
        <button className="select" onClick={() => setOpen((o) => !o)}>
          <span>Change</span>
          <Ms name="expand_more" />
        </button>
        <Popover open={open} onClose={() => setOpen(false)} side="bottom" align="end">
          {devices.map((d) => (
            <div
              key={d.name}
              className={"menu-item" + (d.name === current ? " sel" : "")}
              onClick={() => {
                onPick(d.name);
                setOpen(false);
              }}
            >
              <Ms name={icon} />
              <span>{d.description}</span>
              {d.name === current && <Ms name="check" style={{ marginLeft: "auto" }} />}
            </div>
          ))}
        </Popover>
      </div>
    </div>
  );
}

export function SettingsScreen() {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [backendNative, setBackendNative] = useState<boolean | null>(null);
  const [version, setVersion] = useState("");
  const [defaults, setDefaults] = useState<DefaultDevices>({ output: null, input: null });
  const [error, setError] = useState<string | null>(null);
  const outputDevices = useMixerStore((s) => s.outputDevices);
  const inputDevices = useMixerStore((s) => s.inputDevices);

  useEffect(() => {
    void invoke<boolean>("get_autostart").then(setAutostart);
    void invoke<{ native: boolean }>("get_backend_info").then((i) => setBackendNative(i.native));
    void invoke<DefaultDevices>("get_default_devices").then(setDefaults).catch(() => {});
    void getVersion().then(setVersion);
  }, []);

  const pickDefault = async (kind: "output" | "input", name: string) => {
    try {
      await invoke(kind === "output" ? "set_default_output" : "set_default_input", { name });
      setDefaults((d) => ({ ...d, [kind]: name }));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const toggleAutostart = async () => {
    if (autostart === null) return;
    try {
      const actual = await invoke<boolean>("set_autostart", { enabled: !autostart });
      setAutostart(actual);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="content">
      <div className="screen-head">
        <h1>Settings</h1>
        <div className="sub">Sink behaves the same from the tray</div>
      </div>
      <div className="screen-scroll" style={{ maxWidth: 720 }}>
        {error && <div className="error-banner" style={{ borderRadius: 8 }}>{error}</div>}

        <div className="section-label">Devices</div>
        <div className="card" style={{ padding: "var(--sp-2)" }}>
          <DeviceRow
            icon="speaker"
            title="Default output"
            devices={outputDevices}
            current={defaults.output}
            onPick={(name) => void pickDefault("output", name)}
          />
          <DeviceRow
            icon="mic"
            title="Default input"
            devices={inputDevices}
            current={defaults.input}
            onPick={(name) => void pickDefault("input", name)}
          />
          <div className="empty-hint" style={{ padding: "var(--sp-2) var(--sp-4)", textAlign: "left" }}>
            Channels set to "System default" and the mic chain follow these.
          </div>
        </div>

        <div className="section-label">Startup</div>
        <div className="card" style={{ padding: "var(--sp-2)" }}>
          <div className="row">
            <div className="ricon">
              <Ms name="rocket_launch" />
            </div>
            <div className="rmain">
              <div className="rtitle">Start at login</div>
              <div className="rsub">systemd user service, starts with your desktop session</div>
            </div>
            {autostart !== null && <Toggle on={autostart} onClick={() => void toggleAutostart()} />}
          </div>
        </div>

        <div className="section-label">About</div>
        <div className="card" style={{ padding: "var(--sp-2)" }}>
          <div className="row">
            <div className="ricon">
              <Ms name="cable" />
            </div>
            <div className="rmain">
              <div className="rtitle">Audio engine</div>
              <div className="rsub">
                {backendNative === null
                  ? "…"
                  : backendNative
                    ? "Native PipeWire (pipewire-rs) — live metering, passive routing"
                    : "pactl fallback — native engine unavailable on this system"}
              </div>
            </div>
            {backendNative !== null && (
              <span className={"tag" + (backendNative ? " live" : "")}>
                {backendNative ? "native" : "fallback"}
              </span>
            )}
          </div>
          <div className="row">
            <div className="ricon">
              <Ms name="info" />
            </div>
            <div className="rmain">
              <div className="rtitle">Sink {version}</div>
              <div className="rsub">GPL-3.0 · config in ~/.config/sink</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
