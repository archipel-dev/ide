//! The Terminal tool window's shells as a project service (IntelliJ: `TerminalToolWindowManager`).
//!
//! [`TerminalService`] owns the shell sessions. The active one is [`TerminalState::session`], so
//! everything that types into or reads the terminal works on one session; the others are parked
//! in their tabs and swapped in by [`TerminalState::select_tab`]. Disposing the service (closing
//! the project) drops the sessions, which kills the shells. A shell that ends is announced on
//! [`TERMINAL_TOPIC`] of the project bus the first time polling sees it.

use super::Cell;
use archipel_ide_platform::{Disposable, MessageBus, Topic};
use archipel_ide_terminal::TerminalSession;
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard,
};

/// Reports the life of shell sessions (IntelliJ `TerminalWidgetListener`).
pub trait TerminalListener: Send + Sync {
    /// The shell of the tab called `title` exited.
    fn terminal_exited(&self, _title: &str) {}
}

/// On the project's message bus.
pub const TERMINAL_TOPIC: Topic<dyn TerminalListener> = Topic::new("TerminalListener");

struct Tab {
    id: u64,
    title: String,
    /// `None` for the active tab (its session is `TerminalState::session`).
    session: Option<TerminalSession>,
}

/// The sessions and tabs. Without tabs a lone shell shows as `Local`.
#[derive(Default)]
pub struct TerminalState {
    /// The active session.
    pub session: Option<TerminalSession>,
    tabs: Vec<Tab>,
    active: usize,
    created: usize,
    next_id: u64,
    /// Ids of the sessions whose exit was announced; 0 is the lone shell without tabs.
    announced: Vec<u64>,
}

impl TerminalState {
    fn title(&mut self) -> String {
        self.created += 1;
        match self.created {
            1 => "Local".to_owned(),
            n => format!("Local ({n})"),
        }
    }

    fn new_tab(&mut self, session: Option<TerminalSession>) -> usize {
        self.next_id += 1;
        let title = self.title();
        self.tabs.push(Tab {
            id: self.next_id,
            title,
            session,
        });
        self.tabs.len() - 1
    }

    /// The first session of a Terminal opened without tabs becomes the first tab.
    fn adopt(&mut self) {
        if self.tabs.is_empty() && self.session.is_some() {
            let was_announced = self.announced.contains(&0);
            let index = self.new_tab(None);
            self.announced.retain(|id| *id != 0);
            if was_announced {
                self.announced.push(self.tabs[index].id);
            }
            self.active = 0;
        }
    }

    fn active_id(&self) -> u64 {
        self.tabs.get(self.active).map_or(0, |tab| tab.id)
    }

    /// Replaces the active session (a restart, or a shell opened after the old one exited).
    pub fn set_session(&mut self, session: TerminalSession) {
        let id = self.active_id();
        self.announced.retain(|announced| *announced != id);
        self.session = Some(session);
    }

    /// Tab titles with the active one flagged; a lone shell shows as `Local`.
    pub fn tab_titles(&self) -> Vec<(String, bool)> {
        if self.tabs.is_empty() {
            return if self.session.is_some() {
                vec![("Local".into(), true)]
            } else {
                Vec::new()
            };
        }
        self.tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| (tab.title.clone(), index == self.active))
            .collect()
    }

    /// Adds a tab for `session` and makes it the active one.
    pub fn push_tab(&mut self, session: TerminalSession) {
        self.adopt();
        if let Some(previous) = self.tabs.get_mut(self.active) {
            previous.session = self.session.take();
        }
        self.new_tab(None);
        self.active = self.tabs.len() - 1;
        self.session = Some(session);
    }

    pub fn select_tab(&mut self, index: usize) -> Result<(), String> {
        self.adopt();
        if index >= self.tabs.len() {
            return Err(format!("there is no terminal tab {index}"));
        }
        if index != self.active {
            let next = self.tabs[index].session.take();
            if let Some(previous) = self.tabs.get_mut(self.active) {
                previous.session = self.session.take();
            }
            self.session = next;
            self.active = index;
        }
        Ok(())
    }

    pub fn close_tab(&mut self, index: usize) -> Result<(), String> {
        self.adopt();
        let len = self.tabs.len();
        if index >= len {
            return Err(format!("there is no terminal tab {index}"));
        }
        if len < 2 {
            return Err("the last terminal tab cannot be closed".into());
        }
        if index == self.active {
            // Show the neighbour, then drop the closed (now parked) shell.
            let neighbour = if index + 1 < len {
                index + 1
            } else {
                index - 1
            };
            self.select_tab(neighbour)?;
        }
        let closed = self.tabs.remove(index);
        self.announced.retain(|id| *id != closed.id);
        if self.active > index {
            self.active -= 1;
        }
        Ok(())
    }

    /// Pumps every session so output keeps arriving, also in background tabs; returns the
    /// titles of the shells that exited since the last call.
    fn poll(&mut self) -> Vec<String> {
        let mut exited = Vec::new();
        let active = self.active_id();
        let mut sessions: Vec<(u64, String, &mut TerminalSession)> = Vec::new();
        let lone_title = self.tabs.is_empty().then(|| "Local".to_owned());
        if let Some(session) = self.session.as_mut() {
            let title = lone_title.unwrap_or_else(|| {
                self.tabs
                    .get(self.active)
                    .map(|tab| tab.title.clone())
                    .unwrap_or_default()
            });
            sessions.push((active, title, session));
        }
        for tab in &mut self.tabs {
            if let Some(session) = tab.session.as_mut() {
                sessions.push((tab.id, tab.title.clone(), session));
            }
        }
        for (id, title, session) in sessions {
            session.poll();
            if session.is_exited() && !self.announced.contains(&id) {
                self.announced.push(id);
                exited.push(title);
            }
        }
        exited
    }
}

/// The project service.
pub struct TerminalService {
    bus: MessageBus,
    state: Cell<TerminalState>,
}

impl TerminalService {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: Cell::new(TerminalState::default(), "the terminal sessions"),
        }
    }

    /// Sessions and tabs. Reads nest; hold a write guard briefly.
    pub fn state(&self) -> RwLockReadGuard<'_, TerminalState> {
        self.state.read()
    }

    pub fn state_mut(&self) -> RwLockWriteGuard<'_, TerminalState> {
        self.state.write()
    }

    /// The active shell, if one was started.
    pub fn session(&self) -> MappedRwLockReadGuard<'_, Option<TerminalSession>> {
        self.state.read_map(|state| &state.session)
    }

    /// The active shell for typing into it, resizing and polling.
    pub fn session_mut(&self) -> MappedRwLockWriteGuard<'_, Option<TerminalSession>> {
        self.state.write_map(|state| &mut state.session)
    }

    pub fn has_session(&self) -> bool {
        self.state.read().session.is_some()
    }

    /// Pumps all sessions and announces the shells that exited. Returns whether any session
    /// exists (the UI keeps repainting while one does).
    pub fn poll(&self) -> bool {
        let (exited, any) = {
            let mut state = self.state.write();
            let exited = state.poll();
            let any = state.session.is_some() || state.tabs.iter().any(|t| t.session.is_some());
            (exited, any)
        };
        for title in exited {
            self.bus
                .publish(TERMINAL_TOPIC, |l| l.terminal_exited(&title));
        }
        any
    }
}

impl Disposable for TerminalService {
    /// Kills the shells.
    fn dispose(&self) {
        *self.state.write() = TerminalState::default();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn shell(script: &str) -> TerminalSession {
        TerminalSession::spawn("/bin/sh", &["-c", script], std::path::Path::new("/tmp")).unwrap()
    }

    #[derive(Default)]
    struct Heard(Mutex<Vec<String>>);

    impl TerminalListener for Heard {
        fn terminal_exited(&self, title: &str) {
            self.0.lock().push(title.to_owned());
        }
    }

    fn service() -> (Application, Arc<TerminalService>, Arc<Heard>) {
        let application = Application::new();
        application
            .register_service(ServiceDescriptor::disposable(|app: &Application| {
                TerminalService::new(app.message_bus().clone())
            }))
            .unwrap();
        let heard = Arc::new(Heard::default());
        application
            .message_bus()
            .subscribe(application.disposable(), TERMINAL_TOPIC, heard.clone())
            .unwrap();
        let service = application.service::<TerminalService>();
        (application, service, heard)
    }

    #[test]
    fn sessions_swap_in_and_close() {
        let (_application, service, _) = service();
        service
            .state_mut()
            .set_session(shell("echo first; sleep 30"));
        let mut state = service.state_mut();
        assert_eq!(state.tab_titles(), vec![("Local".to_owned(), true)]);
        assert!(state.close_tab(0).is_err(), "the last tab stays");

        state.push_tab(shell("echo second; sleep 30"));
        assert_eq!(state.tab_titles().len(), 2);
        assert_eq!(state.tab_titles()[1], ("Local (2)".to_owned(), true));

        state.select_tab(0).unwrap();
        assert!(state.tab_titles()[0].1);
        assert!(state.session.is_some());
        assert!(state.select_tab(5).is_err());

        state.close_tab(0).unwrap();
        assert_eq!(state.tab_titles(), vec![("Local (2)".to_owned(), true)]);
        assert!(state.session.is_some());
    }

    #[test]
    fn an_exited_shell_is_announced_once_per_session() {
        let (_application, service, heard) = service();
        service.state_mut().set_session(shell("exit 0"));
        let deadline = Instant::now() + Duration::from_secs(10);
        while heard.0.lock().is_empty() && Instant::now() < deadline {
            service.poll();
            std::thread::sleep(Duration::from_millis(20));
        }
        service.poll();
        assert_eq!(*heard.0.lock(), ["Local"]);

        service.state_mut().set_session(shell("exit 0"));
        while heard.0.lock().len() < 2 && Instant::now() < deadline {
            service.poll();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(
            *heard.0.lock(),
            ["Local", "Local"],
            "a new shell announces again"
        );
    }

    #[test]
    fn disposing_kills_the_shells() {
        let (application, service, _) = service();
        service.state_mut().set_session(shell("sleep 30"));
        assert!(service.has_session());
        application.dispose();
        assert!(!service.has_session());
    }
}
