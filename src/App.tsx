import { useState } from "react";
import { TitleBar } from "./components/TitleBar/TitleBar";
import { MixerBoard } from "./components/MixerBoard/MixerBoard";
import { AppList } from "./components/AppList/AppList";
import { Ms } from "./components/Icons";
import { useAudio } from "./hooks/useAudio";
import { useMixerStore } from "./store/mixer";

// Phase 1 screens. The rail grows as later phases land
// (Routing rules, Sinks, Devices, Mic).
const NAV = [
  { id: "mixer", icon: "graphic_eq", label: "Mixer" },
  { id: "apps", icon: "grid_view", label: "Apps" },
] as const;

type NavId = (typeof NAV)[number]["id"];

export default function App() {
  useAudio();
  const [nav, setNav] = useState<NavId>("mixer");
  const error = useMixerStore((s) => s.error);

  const current = NAV.find((n) => n.id === nav) ?? NAV[0];

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
        </nav>

        {nav === "mixer" ? <MixerBoard /> : <AppList />}
      </div>
    </div>
  );
}
