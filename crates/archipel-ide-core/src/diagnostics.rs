//! Log file and crash reports ("Help → Show Log in Files" in the reference IDE).
//!
//! | OS | Directory |
//! |---|---|
//! | Linux | `$XDG_STATE_HOME/archipel/logs` (default `~/.local/state/archipel/logs`) |
//! | macOS | `~/Library/Logs/Archipel` |
//! | Windows | `%LOCALAPPDATA%\Archipel\logs` |
//!
//! `ARCHIPEL_LOG_DIR` overrides it. `archipel.log` rotates to `archipel.1.log`
//! at 5 MB. A panic writes `crash-<UTC time>-<pid>.txt` with the build, the panic
//! message, a backtrace and the log tail; the next start offers it to the user
//! until dismissed (`crash-reports.seen` remembers the newest one shown).

use crate::build_info::BUILD;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_FILE: &str = "archipel.log";
const ROTATED_LOG_FILE: &str = "archipel.1.log";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const SEEN_FILE: &str = "crash-reports.seen";
const LOG_TAIL_LINES: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

struct Logger {
    dir: PathBuf,
    file: Mutex<File>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Log directory for `os` from environment lookups (testable without touching
/// the real environment).
pub fn log_dir_for(os: &str, env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let var = |name: &str| env(name).filter(|value| !value.is_empty());
    if let Some(dir) = var("ARCHIPEL_LOG_DIR") {
        return Some(PathBuf::from(dir));
    }
    match os {
        "windows" => var("LOCALAPPDATA").map(|base| Path::new(&base).join("Archipel").join("logs")),
        "macos" => var("HOME").map(|home| Path::new(&home).join("Library/Logs/Archipel")),
        _ => var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|home| Path::new(&home).join(".local/state")))
            .map(|state| state.join("archipel").join("logs")),
    }
}

/// This machine's log directory; falls back to the temp directory.
///
/// Logs of the Jetthing versions (`.../jetthing/logs`, `~/Library/Logs/Jetthing`) are moved
/// to the new directory on first use; `JETTHING_LOG_DIR` is still read.
pub fn log_dir() -> PathBuf {
    let Some(dir) = log_dir_for(std::env::consts::OS, |name| {
        archipel_ide_compat::env_var(name).ok()
    }) else {
        return std::env::temp_dir().join("archipel-ide-logs");
    };
    // `<state>/archipel/logs` keeps its logs below the product directory.
    match dir.file_name().is_some_and(|name| name == "logs") {
        true => match dir.parent() {
            Some(parent) => {
                let adopted = archipel_ide_compat::adopt_legacy_dir(parent.to_path_buf());
                if adopted == parent {
                    dir
                } else {
                    adopted.join("logs")
                }
            }
            None => dir,
        },
        false => archipel_ide_compat::adopt_legacy_dir(dir),
    }
}

/// Opens (and rotates) the log file and writes a session header. Safe to call
/// more than once; later calls return the existing directory.
pub fn init() -> io::Result<PathBuf> {
    if let Some(logger) = LOGGER.get() {
        return Ok(logger.dir.clone());
    }
    let dir = log_dir();
    let file = open_log(&dir)?;
    let _ = LOGGER.set(Logger {
        dir: dir.clone(),
        file: Mutex::new(file),
    });
    // Compatibility notices (old environment variables, migrated directories) go to the log.
    archipel_ide_compat::set_warning_hook(|message| log(Level::Warn, message));
    log(
        Level::Info,
        &format!(
            "session start: {} on {} {} (pid {})",
            BUILD.version_line("archipel-ide"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::process::id()
        ),
    );
    Ok(dir)
}

fn open_log(dir: &Path) -> io::Result<File> {
    fs::create_dir_all(dir)?;
    let path = dir.join(LOG_FILE);
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES) {
        let _ = fs::rename(&path, dir.join(ROTATED_LOG_FILE));
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// The log directory once [`init`] succeeded.
pub fn active_log_dir() -> Option<PathBuf> {
    LOGGER.get().map(|logger| logger.dir.clone())
}

/// Appends one line to the log; a no-op before [`init`].
pub fn log(level: Level, message: &str) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    let line = format_line(unix_now(), level, message);
    if let Ok(mut file) = logger.file.lock() {
        let _ = file.write_all(line.as_bytes());
    }
}

fn format_line(seconds: i64, level: Level, message: &str) -> String {
    // One record per line, whatever the message contains.
    let message = message.replace(['\r', '\n'], " ");
    format!("{} {:5} {message}\n", timestamp(seconds), level.label())
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// `YYYY-MM-DD HH:MM:SS` (UTC) for a Unix timestamp.
pub fn timestamp(seconds: i64) -> String {
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    let rest = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// Writes a crash report for a panic into the log directory (initialising it
/// if needed) and returns its path.
pub fn write_crash_report(message: &str, location: Option<&str>) -> io::Result<PathBuf> {
    let dir = active_log_dir().unwrap_or_else(log_dir);
    fs::create_dir_all(&dir)?;
    let now = unix_now();
    let stamp = timestamp(now).replace([' ', ':'], "").replace('-', "");
    let path = dir.join(format!("crash-{stamp}-{}.txt", std::process::id()));
    let backtrace = std::backtrace::Backtrace::force_capture();
    let thread = std::thread::current()
        .name()
        .unwrap_or("<unnamed>")
        .to_owned();
    let log_tail = fs::read_to_string(dir.join(LOG_FILE))
        .map(|text| tail(&text, LOG_TAIL_LINES))
        .unwrap_or_default();
    let report = crash_report_text(&CrashDetails {
        time: &timestamp(now),
        message,
        location,
        thread: &thread,
        backtrace: &backtrace.to_string(),
        log_tail: &log_tail,
    });
    fs::write(&path, report)?;
    log(
        Level::Error,
        &format!("panic: {message}; report {}", path.display()),
    );
    Ok(path)
}

struct CrashDetails<'a> {
    time: &'a str,
    message: &'a str,
    location: Option<&'a str>,
    thread: &'a str,
    backtrace: &'a str,
    log_tail: &'a str,
}

fn crash_report_text(details: &CrashDetails) -> String {
    format!(
        "Archipel IDE crash report\n\
         Time: {} UTC\n\
         {}\
         Thread: {}\n\
         Panic: {}\n\
         Location: {}\n\n\
         Backtrace:\n{}\n\n\
         Last log lines:\n{}",
        details.time,
        BUILD.report().replacen("Archipel IDE ", "Version: ", 1),
        details.thread,
        details.message,
        details.location.unwrap_or("unknown"),
        details.backtrace,
        details.log_tail,
    )
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..]
        .iter()
        .map(|line| format!("{line}\n"))
        .collect()
}

/// Crash reports newer than the last one the user dismissed, newest first.
pub fn unseen_crash_reports(dir: &Path) -> Vec<PathBuf> {
    let seen = fs::read_to_string(dir.join(SEEN_FILE)).unwrap_or_default();
    let seen = seen.trim();
    let mut reports: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("crash-") && name.ends_with(".txt") && name > seen
                })
        })
        .collect();
    reports.sort();
    reports.reverse();
    reports
}

/// Remembers `report` (and everything older) as shown.
pub fn mark_crash_reports_seen(dir: &Path, report: &Path) -> io::Result<()> {
    let name = report
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid report name"))?;
    fs::write(dir.join(SEEN_FILE), name)
}

/// The command that shows `path` in the platform file manager / default app.
pub fn open_command(path: &Path, os: &str) -> std::process::Command {
    let program = match os {
        "windows" => "explorer",
        "macos" => "open",
        _ => "xdg-open",
    };
    let mut command = std::process::Command::new(program);
    command.arg(path);
    command
}

pub fn open_path(path: &Path) -> io::Result<()> {
    open_command(path, std::env::consts::OS).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn log_directory_follows_platform_conventions() {
        let home = [("HOME", "/home/a")];
        assert_eq!(
            log_dir_for("linux", env(&home)),
            Some(PathBuf::from("/home/a/.local/state/archipel/logs"))
        );
        assert_eq!(
            log_dir_for(
                "linux",
                env(&[("HOME", "/home/a"), ("XDG_STATE_HOME", "/s")])
            ),
            Some(PathBuf::from("/s/archipel/logs"))
        );
        assert_eq!(
            log_dir_for("macos", env(&home)),
            Some(PathBuf::from("/home/a/Library/Logs/Archipel"))
        );
        assert_eq!(
            log_dir_for(
                "windows",
                env(&[("LOCALAPPDATA", "C:/Users/a/AppData/Local")])
            ),
            Some(
                Path::new("C:/Users/a/AppData/Local")
                    .join("Archipel")
                    .join("logs")
            )
        );
        assert_eq!(
            log_dir_for(
                "linux",
                env(&[("ARCHIPEL_LOG_DIR", "/tmp/x"), ("HOME", "/h")])
            ),
            Some(PathBuf::from("/tmp/x"))
        );
        assert_eq!(log_dir_for("linux", env(&[("HOME", "")])), None);
    }

    #[test]
    fn timestamps_and_log_lines_are_utc_and_single_line() {
        assert_eq!(timestamp(0), "1970-01-01 00:00:00");
        assert_eq!(timestamp(1_760_100_000), "2025-10-10 12:40:00");
        assert_eq!(timestamp(951_782_400), "2000-02-29 00:00:00");
        assert_eq!(
            format_line(0, Level::Warn, "a\nb\r\nc"),
            "1970-01-01 00:00:00 WARN  a b  c\n"
        );
    }

    #[test]
    fn crash_reports_carry_build_panic_and_log_tail() {
        let text = crash_report_text(&CrashDetails {
            time: "2026-10-10 12:00:00",
            message: "index out of bounds",
            location: Some("crates/x/src/lib.rs:10:5"),
            thread: "main",
            backtrace: "0: frame",
            log_tail: "last line\n",
        });
        assert!(text.starts_with("Archipel IDE crash report\nTime: 2026-10-10 12:00:00 UTC\n"));
        assert!(text.contains(&format!("Version: {}\n", BUILD.version)));
        assert!(text.contains(&format!("Build: {} ({})\n", BUILD.commit, BUILD.date)));
        assert!(text.contains("Panic: index out of bounds\n"));
        assert!(text.contains("Location: crates/x/src/lib.rs:10:5\n"));
        assert!(text.ends_with("Last log lines:\nlast line\n"));
        assert_eq!(tail("a\nb\nc\n", 2), "b\nc\n");
    }

    #[test]
    fn unseen_reports_are_newest_first_until_dismissed() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-crash-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(unseen_crash_reports(&dir).is_empty());
        for name in [
            "crash-20261010T090000-1.txt",
            "crash-20261010T100000-2.txt",
            "archipel.log",
        ] {
            fs::write(dir.join(name), "x").unwrap();
        }
        let reports = unseen_crash_reports(&dir);
        assert_eq!(reports.len(), 2);
        assert!(reports[0].ends_with("crash-20261010T100000-2.txt"));
        mark_crash_reports_seen(&dir, &reports[0]).unwrap();
        assert!(unseen_crash_reports(&dir).is_empty());
        fs::write(dir.join("crash-20261011T080000-3.txt"), "x").unwrap();
        assert_eq!(unseen_crash_reports(&dir).len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rotation_keeps_one_previous_log() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-rotate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let big = vec![b'x'; MAX_LOG_BYTES as usize + 1];
        fs::write(dir.join(LOG_FILE), &big).unwrap();
        let mut file = open_log(&dir).unwrap();
        file.write_all(b"fresh\n").unwrap();
        assert_eq!(fs::read_to_string(dir.join(LOG_FILE)).unwrap(), "fresh\n");
        assert_eq!(
            fs::metadata(dir.join(ROTATED_LOG_FILE)).unwrap().len(),
            big.len() as u64
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn paths_open_with_the_platform_handler() {
        let program = |os| {
            open_command(Path::new("logs"), os)
                .get_program()
                .to_string_lossy()
                .into_owned()
        };
        assert_eq!(program("windows"), "explorer");
        assert_eq!(program("macos"), "open");
        assert_eq!(program("linux"), "xdg-open");
    }
}
