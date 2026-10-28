//! The one-time notice for terminals without the kitty keyboard protocol: the GUI's
//! Ctrl+Shift shortcuts cannot be sent there, so the notice names the leader chords.

use std::path::PathBuf;

/// Whether the notice was shown (this run, or before: a marker file in the config dir).
#[derive(Default)]
pub struct Notice {
    checked: bool,
    marker: Option<PathBuf>,
}

fn marker() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("archipel-ide").join("tui-keyboard-notice"))
}

impl Notice {
    /// Keeps the "shown" marker at `path` instead of the config directory (tests).
    pub fn marker_at(&mut self, path: PathBuf) {
        self.marker = Some(path);
    }

    /// Whether the notice is due now: once per user, only on legacy terminals. Records it.
    pub fn due(&mut self, kitty: bool) -> bool {
        if self.checked {
            return false;
        }
        self.checked = true;
        if kitty {
            return false;
        }
        let Some(path) = self.marker.clone().or_else(marker) else {
            return true;
        };
        if path.exists() {
            return false;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, b"shown\n");
        true
    }
}

/// The text of the notice and of Help > Terminal Keyboard Setup.
pub fn text(leader: &str) -> String {
    format!(
        "This terminal does not report Ctrl+Shift keys, so some shortcuts of the GUI are \
         sent as chords: press {leader}, then a key (for example {leader} E for Search \
         Everywhere).\nTerminals with the kitty keyboard protocol (kitty, WezTerm, foot, \
         Ghostty, Alacritty 0.13+) send every shortcut as in the GUI."
    )
}
