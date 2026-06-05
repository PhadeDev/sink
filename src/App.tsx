import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { TitleBar } from "./components/TitleBar/TitleBar";
import { MixerBoard } from "./components/MixerBoard/MixerBoard";
import { AppList } from "./components/AppList/AppList";
import { MicScreen } from "./components/Mic/MicScreen";
import { OnboardingModal } from "./components/Onboarding/OnboardingModal";
import { SettingsScreen } from "./components/Settings/SettingsScreen";
import { Ms } from "./components/Icons";
import { useAudio } from "./hooks/useAudio";
import { useMixerStore } from "./store/mixer";

const NAV = [
  { id: "mixer", icon: "graphic_eq", label: "Mixer" },
  { id: "apps", icon: "grid_view", label: "Apps" },
  { id: "mic", icon: "mic", label: "Mic" },
] as const;

type NavId = (typeof NAV)[number]["id"] | "settings";

export default function App() {
  useAudio();
  const [nav, setNav] = useState<NavId>("mixer");
  const [version, setVersion] = useState("");
  const error = useMixerStore((s) => s.error);

  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  const current =
    nav === "settings"
      ? { label: "Settings" }
      : (NAV.find((n) => n.id === nav) ?? NAV[0]);

  return (
    <div className="window">
      <TitleBar screen={current.label} />

      {error && (
        <div className="error-banner">
          <strong>Audio error:</strong> {error}
        </div>
      )}

      <div className="body">
        <nav className="rail">
          {NAV.map((n) => (
            <button
              key={n.id}
              className={"nav-item" + (n.id === nav ? " active" : "")}
              onClick={() => setNav(n.id)}
            >
              <Ms name={n.icon} />
              <span className="nav-label">{n.label}</span>
            </button>
          ))}
          <div className="rail-spacer" />
          <button
            className={"nav-item" + (nav === "settings" ? " active" : "")}
            onClick={() => setNav("settings")}
          >
            <Ms name="settings" />
            <span className="nav-label">Settings</span>
          </button>
          {version && <div className="rail-version">v{version}</div>}
        </nav>

        {nav === "mixer" ? (
          <MixerBoard />
        ) : nav === "apps" ? (
          <AppList />
        ) : nav === "mic" ? (
          <MicScreen />
        ) : (
          <SettingsScreen />
        )}
      </div>

      <OnboardingModal />
    </div>
  );
}
