//! Tools that Archipel IDE installed itself (managed JDK, Gradle): their `bin`
//! directories are searched before `PATH`, and spawned tasks get them on
//! `PATH` plus home variables such as `JAVA_HOME`, so builds work on a machine
//! where nothing is installed system-wide. The process environment itself is
//! never modified.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::sync::RwLock;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolEnvironment {
    /// Directories searched before `PATH`, in order.
    pub path_dirs: Vec<PathBuf>,
    /// Variables set for spawned tasks unless the task sets them itself.
    pub vars: Vec<(String, String)>,
}

static TOOL_ENV: RwLock<Option<ToolEnvironment>> = RwLock::new(None);

/// Replaces the managed tool environment (the IDE calls this when toolchains change).
pub fn set_tool_environment(environment: ToolEnvironment) {
    if let Ok(mut current) = TOOL_ENV.write() {
        *current = Some(environment);
    }
}

pub fn tool_environment() -> ToolEnvironment {
    TOOL_ENV
        .read()
        .ok()
        .and_then(|current| current.clone())
        .unwrap_or_default()
}

impl ToolEnvironment {
    /// `PATH` for a child: the managed directories, then `base`.
    pub fn joined_path(&self, base: Option<OsString>) -> Option<OsString> {
        if self.path_dirs.is_empty() {
            return base;
        }
        let rest = base.iter().flat_map(std::env::split_paths);
        std::env::join_paths(self.path_dirs.iter().cloned().chain(rest)).ok()
    }

    /// Applies the environment to `command`; `own` are the task's own variables,
    /// which win over the managed ones.
    pub fn apply(&self, command: &mut Command, own: &[(String, String)]) {
        let sets = |key: &str| own.iter().any(|(name, _)| name == key);
        for (key, value) in &self.vars {
            if !sets(key) {
                command.env(key, value);
            }
        }
        if !sets("PATH")
            && let Some(path) = self.joined_path(std::env::var_os("PATH"))
        {
            command.env("PATH", path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_directories_come_first_on_path() {
        let environment = ToolEnvironment {
            path_dirs: vec![
                PathBuf::from("/opt/jdk/bin"),
                PathBuf::from("/opt/gradle/bin"),
            ],
            vars: vec![("JAVA_HOME".into(), "/opt/jdk".into())],
        };
        let joined = environment
            .joined_path(Some(OsString::from("/usr/bin")))
            .unwrap();
        let parts: Vec<PathBuf> = std::env::split_paths(&joined).collect();
        assert_eq!(
            parts,
            [
                PathBuf::from("/opt/jdk/bin"),
                PathBuf::from("/opt/gradle/bin"),
                PathBuf::from("/usr/bin")
            ]
        );
        assert_eq!(
            ToolEnvironment::default().joined_path(Some("/usr/bin".into())),
            Some(OsString::from("/usr/bin"))
        );
    }
}
