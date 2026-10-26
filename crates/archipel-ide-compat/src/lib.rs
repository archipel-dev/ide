//! Compatibility with the names the IDE had before it became Archipel IDE.
//!
//! This is the one place that knows the old "Jetthing" names. Everything else in the
//! workspace uses the Archipel names and asks this crate when it needs to honour data
//! or configuration written by an older version. Each fallback logs a one-time warning
//! (stderr, and the log file once `archipel_ide_core::diagnostics::init` registered
//! [`set_warning_hook`]).
//!
//! | Old | New | Behaviour |
//! |---|---|---|
//! | `JETTHING_*` environment variables | `ARCHIPEL_*` | the new name wins; the old one is read as a fallback ([`env_var_os`], [`env_var`]) |
//! | `.jetthing/` in a project | `.archipel/` | read from the old directory while the new one is missing; the first write copies the old tree into the new directory ([`project_read_path`], [`project_write_path`]) |
//! | per-user config, data and log directories named `jetthing` / `Jetthing` | `archipel` / `Archipel` | moved (or, if moving fails, copied) to the new name on first use ([`adopt_legacy_dir`]) |
//!
//! The crate has no dependencies and no policy: callers decide when to call it.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Prefix of the old environment variables.
const LEGACY_ENV_PREFIX: &str = "JETTHING_";
/// Prefix of the current environment variables.
const ENV_PREFIX: &str = "ARCHIPEL_";
/// Per-project data directory of older versions.
const LEGACY_PROJECT_DIR: &str = ".jetthing";
/// Per-project data directory.
const PROJECT_DIR: &str = ".archipel";

// ---------------------------------------------------------------------------
// One-time warnings
// ---------------------------------------------------------------------------

static SEEN: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
static PENDING: Mutex<Vec<String>> = Mutex::new(Vec::new());
static HOOK: OnceLock<fn(&str)> = OnceLock::new();

/// Emits `message` once per `key`: on stderr, and to the hook (or the pending list
/// until a hook is registered).
pub fn warn_once(key: &str, message: &str) {
    let first = SEEN
        .lock()
        .map(|mut seen| seen.insert(key.to_owned()))
        .unwrap_or(false);
    if !first {
        return;
    }
    eprintln!("archipel-ide: warning: {message}");
    match HOOK.get() {
        Some(hook) => hook(message),
        None => {
            if let Ok(mut pending) = PENDING.lock() {
                pending.push(message.to_owned());
            }
        }
    }
}

/// Registers the function that receives warnings (the log). Warnings raised before
/// the hook existed are delivered to it right away. Only the first call counts.
pub fn set_warning_hook(hook: fn(&str)) {
    if HOOK.set(hook).is_ok() {
        let pending = PENDING
            .lock()
            .map(|mut pending| std::mem::take(&mut *pending))
            .unwrap_or_default();
        for message in pending {
            hook(&message);
        }
    }
}

// ---------------------------------------------------------------------------
// Environment variables
// ---------------------------------------------------------------------------

/// The old name of `ARCHIPEL_X`, i.e. `JETTHING_X`; `None` for other names.
pub fn legacy_env_name(name: &str) -> Option<String> {
    name.strip_prefix(ENV_PREFIX)
        .map(|rest| format!("{LEGACY_ENV_PREFIX}{rest}"))
}

/// Like [`std::env::var_os`] for an `ARCHIPEL_*` variable, falling back to its old
/// `JETTHING_*` name with a one-time warning.
pub fn env_var_os(name: &str) -> Option<OsString> {
    env_var_os_with(name, |key| std::env::var_os(key))
}

/// [`env_var_os`] with an injectable lookup (for tests and callers with their own
/// environment abstraction).
pub fn env_var_os_with(name: &str, lookup: impl Fn(&str) -> Option<OsString>) -> Option<OsString> {
    if let Some(value) = lookup(name) {
        return Some(value);
    }
    let legacy = legacy_env_name(name)?;
    let value = lookup(&legacy)?;
    warn_once(
        &legacy,
        &format!("the environment variable {legacy} is deprecated; use {name}"),
    );
    Some(value)
}

/// Like [`std::env::var`] for an `ARCHIPEL_*` variable, falling back to its old name.
pub fn env_var(name: &str) -> Result<String, std::env::VarError> {
    match env_var_os(name) {
        Some(value) => value.into_string().map_err(std::env::VarError::NotUnicode),
        None => Err(std::env::VarError::NotPresent),
    }
}

// ---------------------------------------------------------------------------
// Per-user directories
// ---------------------------------------------------------------------------

/// The pre-rename sibling of a per-user directory: same parent, last component
/// `archipel` -> `jetthing`, `Archipel` -> `Jetthing`.
pub fn legacy_sibling(dir: &Path) -> Option<PathBuf> {
    let name = dir.file_name()?.to_str()?;
    let legacy = match name {
        "archipel" => "jetthing",
        "Archipel" => "Jetthing",
        _ => return None,
    };
    Some(dir.with_file_name(legacy))
}

/// Returns `dir`; if it does not exist but the pre-rename sibling does, the sibling
/// is first moved to `dir` (or copied when moving fails). If neither works the old
/// directory is returned so no data is lost. Warns once.
pub fn adopt_legacy_dir(dir: PathBuf) -> PathBuf {
    if dir.exists() {
        return dir;
    }
    let Some(legacy) = legacy_sibling(&dir).filter(|legacy| legacy.is_dir()) else {
        return dir;
    };
    let result = fs::rename(&legacy, &dir).or_else(|_| copy_dir_all(&legacy, &dir));
    match result {
        Ok(()) => {
            warn_once(
                &format!("dir:{}", legacy.display()),
                &format!(
                    "moved the per-user directory {} to {}",
                    legacy.display(),
                    dir.display()
                ),
            );
            dir
        }
        Err(error) => {
            warn_once(
                &format!("dir:{}", legacy.display()),
                &format!(
                    "could not move {} to {} ({error}); still using the old directory",
                    legacy.display(),
                    dir.display()
                ),
            );
            let _ = fs::remove_dir_all(&dir);
            legacy
        }
    }
}

/// Recursively copies `from` to `to` (files and directories; symlinks are copied as
/// the files they point to). `to` must not exist.
pub fn copy_dir_all(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Per-project data directory
// ---------------------------------------------------------------------------

/// Splits `path` at its `.archipel` component: `(project root, rest below it)`.
fn split_project_dir(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let ancestor = path
        .ancestors()
        .find(|ancestor| ancestor.file_name().is_some_and(|name| name == PROJECT_DIR))?;
    let root = ancestor.parent()?.to_path_buf();
    let rest = path.strip_prefix(ancestor).ok()?.to_path_buf();
    Some((root, rest))
}

/// Where to **read** the project file `path` (which names the `.archipel/...`
/// location): the old `.jetthing/...` equivalent while the project has no
/// `.archipel` directory yet and does have a `.jetthing` one; otherwise `path`.
pub fn project_read_path(path: PathBuf) -> PathBuf {
    let Some((root, rest)) = split_project_dir(&path) else {
        return path;
    };
    let legacy = root.join(LEGACY_PROJECT_DIR);
    if !root.join(PROJECT_DIR).exists() && legacy.is_dir() {
        warn_once(
            &format!("project:{}", root.display()),
            &format!(
                "{} uses the old {LEGACY_PROJECT_DIR} data directory; it is copied to {PROJECT_DIR} on the first save",
                root.display()
            ),
        );
        return legacy.join(rest);
    }
    path
}

/// Call before **writing** the project file `path`: the first write into a project
/// that only has a `.jetthing` directory copies that tree to `.archipel` (the old
/// directory stays untouched). Returns `path`.
pub fn project_write_path(path: PathBuf) -> PathBuf {
    if let Some((root, _)) = split_project_dir(&path) {
        let legacy = root.join(LEGACY_PROJECT_DIR);
        let new = root.join(PROJECT_DIR);
        if !new.exists() && legacy.is_dir() {
            match copy_dir_all(&legacy, &new) {
                Ok(()) => warn_once(
                    &format!("project-copy:{}", root.display()),
                    &format!(
                        "copied {} to {} (the old directory was left in place and can be deleted)",
                        legacy.display(),
                        new.display()
                    ),
                ),
                Err(error) => {
                    let _ = fs::remove_dir_all(&new);
                    warn_once(
                        &format!("project-copy:{}", root.display()),
                        &format!(
                            "could not copy {} to {}: {error}",
                            legacy.display(),
                            new.display()
                        ),
                    );
                }
            }
        }
    }
    path
}

// ---------------------------------------------------------------------------
// Plugin ids
// ---------------------------------------------------------------------------

/// Maps an id of a bundled plugin from before the rename (`dev.jetthing.<x>`) to its
/// current id (`dev.archipel.<x>`); other ids are returned unchanged.
pub fn current_plugin_id(id: &str) -> String {
    match id.strip_prefix("dev.jetthing.") {
        Some(rest) => format!("dev.archipel.{rest}"),
        None => id.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archipel-ide-compat-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn env_prefers_the_new_name_and_falls_back_to_the_old_one() {
        let lookup = |key: &str| match key {
            "ARCHIPEL_A" => Some("new".into()),
            "JETTHING_A" | "JETTHING_B" => Some("old".into()),
            _ => None,
        };
        assert_eq!(env_var_os_with("ARCHIPEL_A", lookup), Some("new".into()));
        assert_eq!(env_var_os_with("ARCHIPEL_B", lookup), Some("old".into()));
        assert_eq!(env_var_os_with("ARCHIPEL_C", lookup), None);
        // Names without the prefix have no legacy form.
        assert_eq!(env_var_os_with("HOME", |_| None), None);
        assert_eq!(legacy_env_name("ARCHIPEL_X").as_deref(), Some("JETTHING_X"));
        assert_eq!(legacy_env_name("HOME"), None);
    }

    #[test]
    fn project_dir_reads_the_old_directory_until_the_first_write() {
        let root = temp("project");
        fs::create_dir_all(root.join(".jetthing/runConfigurations")).unwrap();
        fs::write(root.join(".jetthing/settings.toml"), "a = 1").unwrap();
        fs::write(root.join(".jetthing/runConfigurations/x.xml"), "x").unwrap();

        let read = project_read_path(root.join(".archipel/settings.toml"));
        assert_eq!(read, root.join(".jetthing/settings.toml"));
        assert!(!root.join(".archipel").exists());

        let write = project_write_path(root.join(".archipel/settings.toml"));
        assert_eq!(write, root.join(".archipel/settings.toml"));
        assert_eq!(
            fs::read_to_string(root.join(".archipel/runConfigurations/x.xml")).unwrap(),
            "x"
        );
        assert!(
            root.join(".jetthing/settings.toml").is_file(),
            "old dir stays"
        );
        // Once the new directory exists it is authoritative.
        assert_eq!(
            project_read_path(root.join(".archipel/missing.toml")),
            root.join(".archipel/missing.toml")
        );
        // Projects without any data directory are untouched.
        let empty = temp("project-empty");
        assert_eq!(
            project_read_path(empty.join(".archipel/a")),
            empty.join(".archipel/a")
        );
        assert!(!empty.join(".archipel").exists());
        project_write_path(empty.join(".archipel/a"));
        assert!(!empty.join(".archipel").exists());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(empty);
    }

    #[test]
    fn per_user_directory_moves_to_the_new_name() {
        let base = temp("user");
        fs::create_dir_all(base.join("jetthing")).unwrap();
        fs::write(base.join("jetthing/settings.toml"), "a = 1").unwrap();
        let dir = adopt_legacy_dir(base.join("archipel"));
        assert_eq!(dir, base.join("archipel"));
        assert_eq!(
            fs::read_to_string(dir.join("settings.toml")).unwrap(),
            "a = 1"
        );
        assert!(!base.join("jetthing").exists());
        // Existing new directories and unrelated names are left alone.
        fs::create_dir_all(base.join("jetthing")).unwrap();
        assert_eq!(
            adopt_legacy_dir(base.join("archipel")),
            base.join("archipel")
        );
        assert!(base.join("jetthing").exists());
        assert_eq!(adopt_legacy_dir(base.join("other")), base.join("other"));
        assert_eq!(
            legacy_sibling(&base.join("Archipel")),
            Some(base.join("Jetthing"))
        );
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn old_plugin_ids_map_to_the_new_ones() {
        assert_eq!(
            current_plugin_id("dev.jetthing.hello"),
            "dev.archipel.hello"
        );
        assert_eq!(current_plugin_id("com.example.x"), "com.example.x");
    }
}
