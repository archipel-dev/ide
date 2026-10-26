//! Welcome screen: recent projects and Get from VCS (clone with progress).

use crate::{CommandError, IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::{RemoteEvent, RemoteJob, RemoteOperation};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// A running `git clone` started from the welcome screen.
pub struct CloneJob {
    job: RemoteJob,
    pub url: String,
    pub directory: PathBuf,
}

/// Directory name a clone URL suggests: `https://host/org/repo.git` → `repo`.
pub fn repository_name(url: &str) -> String {
    let url = url.trim().trim_end_matches(['/', '\\']);
    let last = url.rsplit(['/', '\\', ':']).next().unwrap_or_default();
    last.strip_suffix(".git").unwrap_or(last).to_owned()
}

/// A JDK the New Project wizard can target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JdkChoice {
    pub major: u32,
    pub label: String,
}

/// `21.0.4+7` → 21, `1.8.0_402` → 8.
pub fn jdk_major(version: &str) -> Option<u32> {
    let mut parts = version
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty());
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        return parts.next()?.parse().ok();
    }
    Some(first)
}

fn release_major(home: &Path) -> Option<u32> {
    let release = std::fs::read_to_string(home.join("release")).ok()?;
    let line = release
        .lines()
        .find(|line| line.starts_with("JAVA_VERSION="))?;
    jdk_major(line.trim_start_matches("JAVA_VERSION=").trim_matches('"'))
}

impl IdeState {
    /// JDKs known to the toolchain registry: the one in use, then managed installs.
    pub fn jdk_choices(&self) -> Vec<JdkChoice> {
        let toolchains = &self.toolchains.toolchains;
        let mut choices: Vec<JdkChoice> = Vec::new();
        let mut add = |major: Option<u32>, label: String| {
            if let Some(major) = major
                && !choices.iter().any(|choice| choice.major == major)
            {
                choices.push(JdkChoice { major, label });
            }
        };
        if let Some(located) = toolchains.locate("jdk") {
            let major = located
                .version
                .as_ref()
                .and_then(|version| jdk_major(version.as_str()))
                .or_else(|| release_major(&located.home));
            add(
                major,
                format!("{} ({})", major.unwrap_or(0), located.home.display()),
            );
        }
        for installed in toolchains.store().installed("jdk") {
            let major = jdk_major(installed.version.as_str());
            add(
                major,
                format!("{} (downloaded)", installed.version.as_str()),
            );
        }
        choices
    }

    /// Persists recent projects in the per-user config directory.
    pub fn load_recent_projects(&mut self) {
        let store =
            archipel_ide_toolchains::config_dir().map(|dir| dir.join("recent-projects.json"));
        self.recent_projects = archipel_ide_project::recent::RecentProjects::load(store);
    }

    pub fn clone_busy(&self) -> bool {
        self.clone_job.is_some()
    }

    pub fn start_clone(&mut self, url: &str, directory: &Path) -> Result<(), String> {
        if self.clone_busy() {
            return Err("Another clone is still running".into());
        }
        let url = url.trim();
        if url.is_empty() {
            return Err("Enter the repository URL".into());
        }
        if directory.exists() {
            return Err(format!("{} already exists", directory.display()));
        }
        let parent = directory
            .parent()
            .filter(|parent| parent.is_dir())
            .ok_or_else(|| format!("The parent of {} does not exist", directory.display()))?;
        let operation = RemoteOperation::Clone {
            url: url.to_owned(),
            directory: directory.display().to_string(),
        };
        let job = RemoteJob::start(parent, operation, self.repaint.notifier())
            .map_err(|error| error.to_string())?;
        self.clone_progress = Some("Cloning…".into());
        self.clone_error = None;
        self.status = "Cloning…".into();
        self.clone_job = Some(CloneJob {
            job,
            url: url.to_owned(),
            directory: directory.to_owned(),
        });
        Ok(())
    }

    /// Applies clone progress; opens the project when the clone succeeds.
    /// Returns true when a new project was opened.
    pub fn poll_clone(&mut self) -> bool {
        let Some(clone) = self.clone_job.as_ref() else {
            return false;
        };
        let mut finished = None;
        for event in clone.job.poll() {
            match event {
                RemoteEvent::Progress(line) => {
                    self.status = format!("Cloning: {line}");
                    self.clone_progress = Some(line);
                }
                RemoteEvent::Finished(result) => finished = Some(result),
            }
        }
        let Some(result) = finished else {
            return false;
        };
        let directory = clone.directory.clone();
        self.clone_job = None;
        self.clone_progress = None;
        match result {
            Ok(_) => match self.open_project(&directory) {
                Ok(()) => true,
                Err(error) => {
                    self.clone_error = Some(error);
                    false
                }
            },
            Err(error) => {
                self.status = "Clone failed".into();
                self.clone_error = Some(error);
                false
            }
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "project.clone",
                "Get from Version Control",
                "Project",
                None,
                r#"{"url":"string","directory":"string"}"#,
            ),
            |state: &IdeState| !state.clone_busy(),
            |state: &mut IdeState, params| {
                let url = string_param(&params, "url")?;
                let directory = string_param(&params, "directory")?;
                state
                    .start_clone(&url, Path::new(directory.trim()))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.removeRecent",
                "Remove from Recent Projects",
                "Project",
                None,
                r#"{"path":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let path = string_param(&params, "path")?;
                Ok(json!({"removed": state.recent_projects.remove(Path::new(&path))}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jdk_majors() {
        assert_eq!(jdk_major("21.0.4+7"), Some(21));
        assert_eq!(jdk_major("1.8.0_402"), Some(8));
        assert_eq!(jdk_major("17"), Some(17));
        assert_eq!(jdk_major("none"), None);
    }

    #[test]
    fn names_repositories_from_urls() {
        for (url, name) in [
            ("https://github.com/org/repo.git", "repo"),
            ("git@github.com:org/tool.git", "tool"),
            ("/srv/git/local/", "local"),
            ("", ""),
        ] {
            assert_eq!(repository_name(url), name, "{url}");
        }
    }

    #[test]
    fn clone_opens_the_project_and_records_it_as_recent() {
        let base =
            std::env::temp_dir().join(format!("archipel-ide-clone-cmd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let origin = base.join("origin");
        std::fs::create_dir_all(&origin).unwrap();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .args(["-c", "user.email=a@b.invalid", "-c", "user.name=T"])
                .args(args)
                .current_dir(&origin)
                .status()
                .unwrap()
                .success();
            assert!(ok, "{args:?}");
        };
        git(&["init", "-q"]);
        std::fs::write(origin.join("a.txt"), "a").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "init"]);

        let commands = crate::built_in_commands();
        let mut state = IdeState::default();
        let target = base.join("work/copy");
        // The parent must exist; clone never creates intermediate directories.
        assert!(
            commands
                .execute(
                    "project.clone",
                    &mut state,
                    json!({"url": origin, "directory": target})
                )
                .is_err()
        );
        let target = base.join("copy");
        commands
            .execute(
                "project.clone",
                &mut state,
                json!({"url": origin, "directory": target}),
            )
            .unwrap();
        for _ in 0..600 {
            if state.poll_clone() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(state.clone_error.is_none(), "{:?}", state.clone_error);
        assert_eq!(
            state.project.as_ref().unwrap().root(),
            target.canonicalize().unwrap()
        );
        assert_eq!(state.recent_projects.entries()[0].name, "copy");
        // A second clone into the same directory is refused with a clear message.
        let error = state.start_clone("x", &target).unwrap_err();
        assert!(error.contains("already exists"), "{error}");
        let _ = std::fs::remove_dir_all(base);
    }
}
