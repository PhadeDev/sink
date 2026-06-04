//! Desktop-entry based icon and name resolution — the same mechanism app
//! launchers use. Parses .desktop files once (Name/Icon/Exec/
//! StartupWMClass), matches streams against them, and resolves icon names
//! to actual files across the freedesktop icon dirs (user, system,
//! Flatpak exports). Results are cached per identity.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
struct DesktopEntry {
    /// Display name, e.g. "Spotify".
    name: String,
    name_lower: String,
    icon: Option<String>,
    /// Basename of the Exec command, lowercased.
    exec_base: Option<String>,
    wm_class_lower: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Resolved {
    /// Absolute path to an icon file, ready for the asset protocol.
    pub icon_path: Option<String>,
    /// Polished display name from the desktop entry, when matched.
    pub display_name: Option<String>,
}

struct Resolver {
    desktops: Vec<DesktopEntry>,
    cache: HashMap<String, Resolved>,
}

static RESOLVER: OnceLock<Mutex<Resolver>> = OnceLock::new();

fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::data_dir() {
        dirs.push(home.join("applications"));
        dirs.push(home.join("flatpak/exports/share/applications"));
    }
    dirs.push(PathBuf::from("/usr/share/applications"));
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs
}

/// Every installed icon theme directory (hicolor first, then whatever
/// themes the distro/user installed — Papirus, Adwaita, breeze, …).
/// Many apps only ship icons inside a theme, so hicolor alone misses them.
fn icon_theme_dirs() -> &'static [PathBuf] {
    // The theme set is stable for the process lifetime; scanning the icon
    // roots once avoids re-walking them for every resolve cache miss
    // (icon_name_to_path tries several candidate names per stream).
    static THEMES: OnceLock<Vec<PathBuf>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let mut roots = Vec::new();
        if let Some(data) = dirs::data_dir() {
            roots.push(data.join("icons"));
            roots.push(data.join("flatpak/exports/share/icons"));
        }
        roots.push(PathBuf::from("/usr/share/icons"));
        roots.push(PathBuf::from("/var/lib/flatpak/exports/share/icons"));

        let mut themes = Vec::new();
        for root in roots {
            // hicolor is the freedesktop fallback theme — search it first.
            let hicolor = root.join("hicolor");
            if hicolor.is_dir() {
                themes.push(hicolor);
            }
            if let Ok(read) = fs::read_dir(&root) {
                for entry in read.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.file_name().is_some_and(|n| n != "hicolor") {
                        themes.push(path);
                    }
                }
            }
        }
        themes
    })
}

fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
    let raw = fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let (mut name, mut icon, mut exec, mut wm_class, mut no_display) =
        (None::<String>, None, None, None, false);
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "Name" if name.is_none() => name = Some(value.to_string()),
                "Icon" => icon = Some(value.to_string()),
                "Exec" => exec = Some(value.to_string()),
                "StartupWMClass" => wm_class = Some(value.to_string()),
                "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }
    if no_display {
        return None;
    }
    let name = name?;
    let exec_base = exec.and_then(|e| {
        let first = e.split_whitespace().next()?;
        Path::new(first)
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
    });
    Some(DesktopEntry {
        name_lower: name.to_lowercase(),
        name,
        icon,
        exec_base,
        wm_class_lower: wm_class.map(|w| w.to_lowercase()),
    })
}

fn load_desktops() -> Vec<DesktopEntry> {
    let mut entries = Vec::new();
    for dir in desktop_dirs() {
        let Ok(read) = fs::read_dir(&dir) else { continue };
        for file in read.flatten() {
            let path = file.path();
            if path.extension().is_some_and(|e| e == "desktop") {
                if let Some(entry) = parse_desktop_file(&path) {
                    entries.push(entry);
                }
            }
        }
    }
    entries
}

/// Resolve an icon name to a file path across the freedesktop dirs.
fn icon_name_to_path(name: &str) -> Option<String> {
    if name.starts_with('/') && Path::new(name).exists() {
        return Some(name.to_string());
    }
    const SIZES: [&str; 9] = [
        "64x64", "128x128", "256x256", "96x96", "72x72", "48x48", "512x512", "32x32", "24x24",
    ];
    for theme in icon_theme_dirs() {
        for size in SIZES {
            let p = theme.join(size).join("apps").join(format!("{name}.png"));
            if p.exists() {
                return Some(p.to_string_lossy().into_owned());
            }
            // Some themes nest the size the other way around (apps/<size>).
            let p = theme.join("apps").join(size).join(format!("{name}.svg"));
            if p.exists() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
        let svg = theme.join("scalable/apps").join(format!("{name}.svg"));
        if svg.exists() {
            return Some(svg.to_string_lossy().into_owned());
        }
    }
    for ext in ["png", "svg", "xpm"] {
        let p = PathBuf::from("/usr/share/pixmaps").join(format!("{name}.{ext}"));
        if p.exists() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}

/// Resolve the best icon path + display name for a stream.
///
/// `binary` is the process binary when the identity came from it;
/// `icon_hint` is the stream's application.icon-name property.
pub fn resolve(app_name: &str, binary: Option<&str>, icon_hint: Option<&str>) -> Resolved {
    let resolver = RESOLVER.get_or_init(|| {
        Mutex::new(Resolver {
            desktops: load_desktops(),
            cache: HashMap::new(),
        })
    });
    let Ok(mut resolver) = resolver.lock() else {
        return Resolved::default();
    };

    let key = format!("{app_name}\0{binary:?}\0{icon_hint:?}");
    if let Some(hit) = resolver.cache.get(&key) {
        return hit.clone();
    }

    let app_lower = app_name.to_lowercase();
    let binary_lower = binary.map(str::to_lowercase);
    let desktop = resolver.desktops.iter().find(|d| {
        d.wm_class_lower.as_deref() == Some(app_lower.as_str())
            || (binary_lower.is_some() && d.exec_base == binary_lower)
            || d.name_lower == app_lower
            || d.exec_base.as_deref() == Some(app_lower.as_str())
    });

    // Icon candidates in priority order: explicit stream hint, the desktop
    // entry's icon, the binary name, a slug of the display name.
    let slug = app_lower.replace(' ', "-");
    let mut candidates: Vec<&str> = Vec::new();
    if let Some(hint) = icon_hint {
        candidates.push(hint);
    }
    if let Some(d) = desktop {
        if let Some(icon) = d.icon.as_deref() {
            candidates.push(icon);
        }
    }
    if let Some(b) = binary_lower.as_deref() {
        candidates.push(b);
    }
    candidates.push(&slug);

    let resolved = Resolved {
        icon_path: candidates.iter().find_map(|c| icon_name_to_path(c)),
        display_name: desktop.map(|d| d.name.clone()),
    };
    resolver.cache.insert(key, resolved.clone());
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_desktop_entry() {
        let dir = std::env::temp_dir().join("sink-test-desktop");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test.desktop");
        fs::write(
            &path,
            "[Desktop Entry]\nName=Cool App\nExec=/usr/bin/coolapp --flag\nIcon=coolapp\nStartupWMClass=CoolApp\n",
        )
        .expect("writes");
        let entry = parse_desktop_file(&path).expect("parses");
        assert_eq!(entry.name, "Cool App");
        assert_eq!(entry.exec_base.as_deref(), Some("coolapp"));
        assert_eq!(entry.wm_class_lower.as_deref(), Some("coolapp"));
        assert_eq!(entry.icon.as_deref(), Some("coolapp"));
    }

    #[test]
    fn nodisplay_entries_are_skipped() {
        let dir = std::env::temp_dir().join("sink-test-desktop");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("hidden.desktop");
        fs::write(&path, "[Desktop Entry]\nName=Hidden\nNoDisplay=true\n").expect("writes");
        assert!(parse_desktop_file(&path).is_none());
    }
}
