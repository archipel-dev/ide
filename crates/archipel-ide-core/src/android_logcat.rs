//! Logcat tool window state and `android.logcat.*` commands.

use crate::{IdeState, spec, string_param};
use archipel_ide_android::logcat::started_process;
use archipel_ide_android::{FilterContext, LogEntry, LogFilter, LogcatEvent, LogcatSession};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};

/// Entries kept per session; older lines are dropped first.
pub const LOGCAT_CAPACITY: usize = 100_000;

/// Default query, like the reference IDE: only the project's app.
pub const DEFAULT_QUERY: &str = "package:mine";

/// Entries kept in the filter history.
pub const HISTORY_CAPACITY: usize = 20;

/// One Logcat tab: a device plus a filter. All tabs share the single live
/// stream; selecting a tab restarts it for the tab's device and filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogcatTab {
    pub id: u32,
    pub serial: Option<String>,
    pub query: String,
}

pub struct LogcatState {
    pub tabs: Vec<LogcatTab>,
    pub active_tab: usize,
    next_tab_id: u32,
    /// Saved filter queries (the star menu).
    pub favourites: Vec<String>,
    /// Recently committed filter queries, newest first.
    pub history: Vec<String>,
    session: Option<LogcatSession>,
    /// Device the Logcat window shows.
    pub serial: Option<String>,
    entries: VecDeque<LogEntry>,
    /// Number of entries dropped from the front (keeps ids stable).
    dropped: usize,
    /// Ids of entries passing the filter, ascending.
    visible: Vec<usize>,
    /// Entries received while paused.
    pending: Vec<LogEntry>,
    pub processes: HashMap<u32, String>,
    pub query: String,
    pub match_case: bool,
    filter: LogFilter,
    pub filter_error: Option<String>,
    pub paused: bool,
    pub soft_wrap: bool,
    pub error: Option<String>,
    /// Application IDs for `package:mine`.
    pub mine: Vec<String>,
}

impl Default for LogcatState {
    fn default() -> Self {
        Self {
            tabs: vec![LogcatTab {
                id: 1,
                serial: None,
                query: DEFAULT_QUERY.into(),
            }],
            active_tab: 0,
            next_tab_id: 2,
            favourites: Vec::new(),
            history: Vec::new(),
            session: None,
            serial: None,
            entries: VecDeque::new(),
            dropped: 0,
            visible: Vec::new(),
            pending: Vec::new(),
            processes: HashMap::new(),
            query: DEFAULT_QUERY.into(),
            match_case: false,
            filter: LogFilter::parse(DEFAULT_QUERY, false).unwrap_or_default(),
            filter_error: None,
            paused: false,
            soft_wrap: false,
            error: None,
            mine: Vec::new(),
        }
    }
}

impl LogcatState {
    pub fn is_running(&self) -> bool {
        self.session.is_some()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entries passing the current filter, oldest first.
    pub fn visible(&self) -> impl ExactSizeIterator<Item = &LogEntry> + DoubleEndedIterator {
        self.visible
            .iter()
            .map(|id| &self.entries[id - self.dropped])
    }

    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    pub fn visible_entry(&self, index: usize) -> Option<&LogEntry> {
        let id = *self.visible.get(index)?;
        self.entries.get(id - self.dropped)
    }

    pub fn process_name(&self, pid: u32) -> Option<&str> {
        self.processes.get(&pid).map(String::as_str)
    }

    fn passes(&self, entry: &LogEntry) -> bool {
        let lookup = |pid: u32| self.process_name(pid);
        self.filter.matches(
            entry,
            &FilterContext {
                mine: &self.mine,
                process_name: &lookup,
            },
        )
    }

    fn refilter(&mut self) {
        let visible = (0..self.entries.len())
            .filter(|index| self.passes(&self.entries[*index]))
            .map(|index| index + self.dropped)
            .collect();
        self.visible = visible;
    }

    fn append(&mut self, entries: Vec<LogEntry>) {
        let mut learned = false;
        for entry in entries {
            if let Some((pid, name)) = started_process(&entry) {
                learned |= self.processes.insert(pid, name.clone()).as_ref() != Some(&name);
            }
            let id = self.dropped + self.entries.len();
            if !learned && self.passes(&entry) {
                self.visible.push(id);
            }
            self.entries.push_back(entry);
            if learned {
                // A new process mapping can change earlier `package:` results.
                self.refilter();
                learned = false;
            }
        }
        let excess = self.entries.len().saturating_sub(LOGCAT_CAPACITY);
        if excess > 0 {
            self.entries.drain(..excess);
            self.dropped += excess;
            let first = self.dropped;
            self.visible.retain(|id| *id >= first);
        }
    }

    pub fn set_query(&mut self, query: &str, match_case: bool) {
        self.query = query.to_owned();
        self.match_case = match_case;
        match LogFilter::parse(query, match_case) {
            Ok(filter) => {
                self.filter = filter;
                self.filter_error = None;
                self.refilter();
            }
            Err(error) => self.filter_error = Some(error),
        }
    }

    /// Remembers a committed query at the front of the history.
    pub fn remember_query(&mut self, query: &str) {
        let query = query.trim();
        if query.is_empty() {
            return;
        }
        self.history.retain(|q| q != query);
        self.history.insert(0, query.to_owned());
        self.history.truncate(HISTORY_CAPACITY);
    }

    /// Adds `query` to the favourites, or removes it when present.
    pub fn toggle_favourite(&mut self, query: &str) -> bool {
        let query = query.trim();
        if let Some(index) = self.favourites.iter().position(|q| q == query) {
            self.favourites.remove(index);
            false
        } else if query.is_empty() {
            false
        } else {
            self.favourites.push(query.to_owned());
            true
        }
    }

    /// Saves the live device and filter into the active tab.
    fn sync_active_tab(&mut self) {
        let (serial, query) = (self.serial.clone(), self.query.clone());
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.serial = serial;
            tab.query = query;
        }
    }

    /// Opens a new tab copying the live device, with the default filter.
    pub fn new_tab(&mut self) {
        self.sync_active_tab();
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(LogcatTab {
            id,
            serial: self.serial.clone(),
            query: DEFAULT_QUERY.into(),
        });
        self.active_tab = self.tabs.len() - 1;
        let match_case = self.match_case;
        self.set_query(DEFAULT_QUERY, match_case);
    }

    /// Closes tab `index`; the last tab stays. Returns the tab to show.
    pub fn close_tab(&mut self, index: usize) -> Option<LogcatTab> {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return None;
        }
        self.sync_active_tab();
        self.tabs.remove(index);
        if self.active_tab > index || self.active_tab >= self.tabs.len() {
            self.active_tab = self.active_tab.saturating_sub(1);
        }
        self.tabs.get(self.active_tab).cloned()
    }

    /// Activates tab `index`, returning it so the caller can restart the stream.
    pub fn select_tab(&mut self, index: usize) -> Option<LogcatTab> {
        if index >= self.tabs.len() {
            return None;
        }
        self.sync_active_tab();
        self.active_tab = index;
        let tab = self.tabs[index].clone();
        let match_case = self.match_case;
        self.set_query(&tab.query, match_case);
        Some(tab)
    }

    pub fn clear(&mut self) {
        self.dropped += self.entries.len();
        self.entries.clear();
        self.visible.clear();
        self.pending.clear();
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if !paused {
            let pending = std::mem::take(&mut self.pending);
            self.append(pending);
        }
    }

    pub fn stop(&mut self) {
        self.session = None;
    }

    /// Drains session events. Returns whether anything changed.
    pub fn poll(&mut self) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        let events = session.poll();
        let changed = !events.is_empty();
        for event in events {
            match event {
                LogcatEvent::Entries(entries) if self.paused => self.pending.extend(entries),
                LogcatEvent::Entries(entries) => self.append(entries),
                LogcatEvent::Processes(processes) => {
                    let mut changed = false;
                    for (pid, name) in processes {
                        changed |= self.processes.get(&pid) != Some(&name);
                        self.processes.insert(pid, name);
                    }
                    if changed && self.filter.uses_packages() {
                        self.refilter();
                    }
                }
                LogcatEvent::Ended(message) => {
                    self.session = None;
                    self.error = Some(message);
                }
            }
        }
        changed
    }
}

impl IdeState {
    /// Starts Logcat for `serial`, or for the selected device.
    pub fn start_logcat(&mut self, serial: Option<String>) -> Result<(), String> {
        let serial = serial
            .or_else(|| self.android.devices.selected.clone())
            .ok_or("No connected devices. Connect a device or start an emulator.")?;
        let adb = self.android.adb()?;
        let session = LogcatSession::start(&adb, &serial)?;
        let logcat = &mut self.android.logcat;
        if logcat.serial.as_deref() != Some(serial.as_str()) {
            logcat.clear();
            logcat.processes.clear();
        }
        if let Some(project) = &self.android.project {
            logcat.mine = project
                .modules
                .iter()
                .filter_map(|m| m.package_name().map(str::to_owned))
                .collect();
        }
        logcat.serial = Some(serial);
        logcat.error = None;
        logcat.session = Some(session);
        logcat.refilter();
        Ok(())
    }

    pub(crate) fn logcat_opened(&mut self) {
        if !self.android.logcat.is_running() {
            let _ = self.android.ensure_device_tracking();
            if let Err(error) = self.start_logcat(None) {
                self.android.logcat.error = Some(error);
            }
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.logcat.start",
                "Start Logcat",
                "Android",
                None,
                r#"{"serial":"string?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let serial = params
                    .get("serial")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                state.start_logcat(serial).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.logcat.filter",
                "Filter Logcat",
                "Android",
                None,
                r#"{"query":"string","matchCase":false,"remember":false}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let query = string_param(&params, "query")?;
                let match_case = params
                    .get("matchCase")
                    .and_then(Value::as_bool)
                    .unwrap_or(state.android.logcat.match_case);
                let logcat = &mut state.android.logcat;
                logcat.set_query(&query, match_case);
                if let Some(error) = &logcat.filter_error {
                    return Err(CommandError::InvalidParams(error.clone()));
                }
                if params.get("remember").and_then(Value::as_bool) == Some(true) {
                    logcat.remember_query(&query);
                }
                logcat.sync_active_tab();
                Ok(json!({"visible": logcat.visible_len(), "total": logcat.len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.logcat.tab.new",
                "New Logcat Tab",
                "Android",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.android.logcat.new_tab();
                Ok(json!({"tabs": state.android.logcat.tabs.len()}))
            },
        ))
        .expect("unique built-in id");
    for (id, label) in [
        ("android.logcat.tab.select", "Select Logcat Tab"),
        ("android.logcat.tab.close", "Close Logcat Tab"),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Android", None, r#"{"index":0}"#),
                |_| true,
                move |state: &mut IdeState, params| {
                    let index =
                        params.get("index").and_then(Value::as_u64).ok_or_else(|| {
                            CommandError::InvalidParams("index is required".into())
                        })? as usize;
                    let logcat = &mut state.android.logcat;
                    let tab = if id == "android.logcat.tab.select" {
                        logcat.select_tab(index)
                    } else {
                        logcat.close_tab(index)
                    };
                    let Some(tab) = tab else {
                        return Err(CommandError::InvalidParams(format!(
                            "no Logcat tab {index} (or it is the last one)"
                        )));
                    };
                    if tab.serial != state.android.logcat.serial
                        && let Some(serial) = tab.serial.clone()
                    {
                        state
                            .start_logcat(Some(serial))
                            .map_err(CommandError::Failed)?;
                    }
                    Ok(json!({"active": state.android.logcat.active_tab}))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "android.logcat.favourite",
                "Toggle Logcat Filter Favourite",
                "Android",
                None,
                r#"{"query":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let query = string_param(&params, "query")?;
                let added = state.android.logcat.toggle_favourite(&query);
                Ok(json!({"favourite": added}))
            },
        ))
        .expect("unique built-in id");
    for (id, label) in [
        ("android.logcat.clear", "Clear Logcat"),
        ("android.logcat.pause", "Pause Logcat"),
        ("android.logcat.softWrap", "Soft-Wrap Logcat"),
        ("android.logcat.stop", "Stop Logcat"),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Android", None, "{}"),
                |_| true,
                move |state: &mut IdeState, _| {
                    let logcat = &mut state.android.logcat;
                    match id {
                        "android.logcat.clear" => logcat.clear(),
                        "android.logcat.pause" => logcat.set_paused(!logcat.paused),
                        "android.logcat.softWrap" => logcat.soft_wrap = !logcat.soft_wrap,
                        _ => logcat.stop(),
                    }
                    Ok(json!({"paused": logcat.paused, "softWrap": logcat.soft_wrap}))
                },
            ))
            .expect("unique built-in id");
    }
}

#[cfg(test)]
mod tab_tests {
    use super::*;

    #[test]
    fn tabs_history_and_favourites() {
        let mut logcat = LogcatState::default();
        logcat.new_tab();
        assert_eq!(logcat.tabs.len(), 2);
        logcat.set_query("level:warn", false);
        logcat.sync_active_tab();
        let first = logcat.select_tab(0).unwrap();
        assert_eq!(first.query, DEFAULT_QUERY);
        assert_eq!(logcat.query, DEFAULT_QUERY);
        assert_eq!(logcat.select_tab(1).unwrap().query, "level:warn");
        assert!(logcat.close_tab(1).is_some());
        assert!(logcat.close_tab(0).is_none(), "the last tab stays");
        logcat.remember_query("tag:A");
        logcat.remember_query("tag:B");
        logcat.remember_query("tag:A");
        assert_eq!(logcat.history, ["tag:A", "tag:B"]);
        assert!(logcat.toggle_favourite("tag:A"));
        assert!(!logcat.toggle_favourite("tag:A"));
        assert!(logcat.favourites.is_empty());
    }
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-core-logcat-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const LOG: &str = "10-10 14:01:02.345  1234  1250 I ActivityManager: Start proc 4321:com.example.app/u0a190 for activity\n\
10-10 14:01:02.500  4321  4321 D MainActivity: onCreate\n\
10-10 14:01:03.000  4321  4321 E AndroidRuntime: FATAL EXCEPTION: main\n\
10-10 14:01:03.100   777   777 W chatty  : identical 2 lines\n";

    fn fake_sdk(root: &Path) {
        let payload = "emulator-5554\tdevice\n";
        let script = format!(
            "#!/bin/sh\ncase \"$*\" in\n  \"track-devices -l\") printf '%s' '{:04x}{payload}'; exec sleep 30 ;;\n  \"-s emulator-5554 logcat -v threadtime\") printf '%s' '{LOG}'; exec sleep 30 ;;\n  \"-s emulator-5554 shell ps -A -o PID,NAME\") printf '  PID NAME\\n  777 logd\\n' ;;\n  *) exit 1 ;;\nesac\n",
            payload.len()
        );
        let adb = root.join("platform-tools/adb");
        std::fs::create_dir_all(adb.parent().unwrap()).unwrap();
        std::fs::write(&adb, script).unwrap();
        std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn wait_until(state: &mut IdeState, condition: impl Fn(&IdeState) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition(state) {
            assert!(Instant::now() < deadline, "timed out");
            state.poll_android();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn logcat_window_streams_filters_pauses_and_clears() {
        let base = temp("stream");
        let sdk = base.join("sdk");
        fake_sdk(&sdk);
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.set_android_sdk(&sdk).unwrap();
        state.android.ensure_device_tracking().unwrap();
        wait_until(&mut state, |s| s.android.devices.selected.is_some());
        state.android.logcat.mine = vec!["com.example.app".into()];
        commands
            .execute("toolwindow.activate.logcat", &mut state, json!({}))
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::Logcat));
        wait_until(&mut state, |s| s.android.logcat.len() == 4);
        // `package:mine` (default) resolves pid 4321 from the "Start proc" line.
        let tags: Vec<_> = state
            .android
            .logcat
            .visible()
            .map(|e| e.tag.clone())
            .collect();
        assert_eq!(tags, ["MainActivity", "AndroidRuntime"]);
        let result = commands
            .execute(
                "android.logcat.filter",
                &mut state,
                json!({"query": "level:warn"}),
            )
            .unwrap();
        assert_eq!(result, json!({"visible": 2, "total": 4}));
        wait_until(&mut state, |s| {
            s.android.logcat.process_name(777) == Some("logd")
        });
        commands
            .execute(
                "android.logcat.filter",
                &mut state,
                json!({"query": "package:logd"}),
            )
            .unwrap();
        assert_eq!(state.android.logcat.visible_len(), 1);
        assert!(
            commands
                .execute(
                    "android.logcat.filter",
                    &mut state,
                    json!({"query": "tag~:("})
                )
                .is_err()
        );
        assert_eq!(
            state.android.logcat.visible_len(),
            1,
            "invalid query keeps the last filter"
        );

        commands
            .execute("android.logcat.pause", &mut state, json!({}))
            .unwrap();
        assert!(state.android.logcat.paused);
        commands
            .execute("android.logcat.clear", &mut state, json!({}))
            .unwrap();
        assert!(state.android.logcat.is_empty());
        commands
            .execute("android.logcat.softWrap", &mut state, json!({}))
            .unwrap();
        assert!(state.android.logcat.soft_wrap);
        commands
            .execute("android.logcat.stop", &mut state, json!({}))
            .unwrap();
        assert!(!state.android.logcat.is_running());
        drop(state);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn logcat_without_devices_explains_why() {
        let mut state = IdeState::default();
        let error = state.start_logcat(None).unwrap_err();
        assert!(error.contains("No connected devices"), "{error}");
    }

    #[test]
    fn capacity_drops_oldest_entries() {
        let mut logcat = super::LogcatState::default();
        logcat.set_query("", false);
        let entry =
            archipel_ide_android::logcat::parse_threadtime("10-10 14:01:02.345  1  1 I T: m")
                .unwrap();
        logcat.append(vec![entry; super::LOGCAT_CAPACITY + 10]);
        assert_eq!(logcat.len(), super::LOGCAT_CAPACITY);
        assert_eq!(logcat.visible_len(), super::LOGCAT_CAPACITY);
        assert!(logcat.visible_entry(0).is_some());
        assert!(logcat.visible_entry(super::LOGCAT_CAPACITY - 1).is_some());
    }
}
