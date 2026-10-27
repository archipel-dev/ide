//! Breakpoints and the debug session as a project service (IntelliJ: `XDebuggerManager`).
//!
//! [`XDebuggerManager`] owns the breakpoints, their options, the watch expressions, the
//! dialogs' state and the running [`DebugClient`] (or the session still being prepared). The
//! life of a session is announced on [`DEBUG_SESSION_TOPIC`] of the project bus: started,
//! paused at a new execution point, resumed, stopped. The editor reveals the execution point
//! by listening, not by being called from the polling code.

use super::Cell;
use crate::debug_commands::DebuggerState;
use archipel_ide_dap::{DebugClient, SessionState, StackFrame};
use archipel_ide_platform::{Disposable, MessageBus, Topic};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

/// Reports the life of the debug session (IntelliJ `XDebugSessionListener`).
pub trait DebugSessionListener: Send + Sync {
    /// The adapter was started for the configuration called `name`.
    fn session_started(&self, _name: &str) {}

    /// Execution stopped at a point not seen before (a breakpoint, a step, a pause). The
    /// manager is not locked while listeners run, so they may read the frame.
    fn session_paused(&self, _name: &str, _frame: &StackFrame) {}

    /// The paused session runs again.
    fn session_resumed(&self, _name: &str) {}

    /// The program ended or the adapter stopped.
    fn session_stopped(&self, _name: &str) {}
}

/// On the project's message bus.
pub const DEBUG_SESSION_TOPIC: Topic<dyn DebugSessionListener> = Topic::new("DebugSessionListener");

/// What was last announced about the session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Announced {
    #[default]
    NoSession,
    Running,
    Paused,
}

/// The project service. Reads nest; hold a write guard briefly.
pub struct XDebuggerManager {
    bus: MessageBus,
    state: Cell<DebuggerState>,
}

enum Event {
    Paused(StackFrame),
    Resumed,
    Stopped,
}

impl XDebuggerManager {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: Cell::new(DebuggerState::default(), "the debugger state"),
        }
    }

    /// Breakpoints, options, watches, dialogs and the session.
    pub fn state(&self) -> RwLockReadGuard<'_, DebuggerState> {
        self.state.read()
    }

    pub fn state_mut(&self) -> RwLockWriteGuard<'_, DebuggerState> {
        self.state.write()
    }

    /// A session is running, or one is still waiting for its build or for the Java debugger.
    pub fn is_debugging(&self) -> bool {
        let state = self.state.read();
        state
            .client
            .as_ref()
            .is_some_and(|client| !client.session().is_terminated())
            || state.pending.is_some()
            || state.java.is_some()
    }

    /// Forgets everything the previous project had (breakpoints load afterwards).
    pub fn reset(&self) {
        *self.state.write() = DebuggerState::default();
    }

    /// Makes `client` the debug session of the configuration `name` and announces it.
    pub fn attach_session(&self, name: &str, client: DebugClient) {
        {
            let mut state = self.state.write();
            state.name = name.to_owned();
            state.client = Some(client);
            state.reset_revealed();
            state.announced = Announced::Running;
        }
        self.bus
            .publish(DEBUG_SESSION_TOPIC, |l| l.session_started(name));
    }

    /// Pumps the adapter and announces what changed since the last call.
    pub fn poll_session(&self) {
        let (name, events) = {
            let mut state = self.state.write();
            let Some(client) = state.client.as_mut() else {
                return;
            };
            client.poll();
            let session = client.session();
            let error = session.error().map(str::to_owned);
            let stopped_at = match session.state() {
                SessionState::Stopped { thread_id, .. } => session
                    .selected_frame()
                    .map(|frame| (*thread_id, frame.clone())),
                _ => None,
            };
            let terminated = session.is_terminated();
            let stopped = matches!(session.state(), SessionState::Stopped { .. });
            if let Some(error) = error {
                state.error = Some(error);
            }
            let mut events = Vec::new();
            match stopped_at {
                Some((thread_id, frame)) => {
                    if state.revealed() != Some((thread_id, frame.id)) {
                        state.set_revealed(Some((thread_id, frame.id)));
                        state.announced = Announced::Paused;
                        events.push(Event::Paused(frame));
                    }
                }
                None if !stopped => {
                    state.set_revealed(None);
                    if state.announced == Announced::Paused && !terminated {
                        state.announced = Announced::Running;
                        events.push(Event::Resumed);
                    }
                }
                None => {}
            }
            if terminated {
                if state.announced != Announced::NoSession {
                    events.push(Event::Stopped);
                }
                state.announced = Announced::NoSession;
            }
            (state.name.clone(), events)
        };
        for event in events {
            self.bus.publish(DEBUG_SESSION_TOPIC, |l| match &event {
                Event::Paused(frame) => l.session_paused(&name, frame),
                Event::Resumed => l.session_resumed(&name),
                Event::Stopped => l.session_stopped(&name),
            });
        }
    }
}

impl Disposable for XDebuggerManager {
    /// Ends the session (the client's drop stops the adapter) and forgets the breakpoints.
    fn dispose(&self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_commands::PendingDebug;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use std::path::PathBuf;

    #[test]
    fn a_session_waiting_for_its_build_counts_and_disposal_forgets_the_state() {
        let application = Application::new();
        application
            .register_service(ServiceDescriptor::disposable(|app: &Application| {
                XDebuggerManager::new(app.message_bus().clone())
            }))
            .unwrap();
        let debugger = application.service::<XDebuggerManager>();
        assert!(!debugger.is_debugging());
        debugger
            .state_mut()
            .breakpoints
            .entry(PathBuf::from("a.c"))
            .or_default()
            .insert(3);
        debugger.state_mut().pending = Some(PendingDebug {
            name: "app".into(),
            program: PathBuf::from("app"),
            args: Vec::new(),
            cwd: PathBuf::from("."),
            env: Vec::new(),
        });
        assert!(debugger.is_debugging());
        // Polling without a client announces nothing and changes nothing.
        debugger.poll_session();
        assert_eq!(debugger.state().announced, Announced::NoSession);

        application.dispose();
        assert!(!debugger.is_debugging());
        assert!(debugger.state().breakpoints.is_empty());
    }
}
