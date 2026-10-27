//! Notification balloons as an application service (IntelliJ: `NotificationsManager`).
//!
//! Features report a balloon here; the UI shows what the service holds and dismisses it. The
//! service announces new balloons on [`NOTIFICATION_TOPIC`], so listeners (a tool window
//! stripe badge, a test) need no access to the UI.

use archipel_ide_platform::{MessageBus, Topic};
use archipel_ide_plugins::Notification;
use parking_lot::Mutex;

/// Plugin balloons kept; older ones are dropped.
const MAX_PLUGIN_NOTIFICATIONS: usize = 20;

/// Where a balloon came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationSource {
    /// The "update available" balloon.
    Update,
    /// A plugin's `notify`, with the plugin id.
    Plugin(String),
}

pub trait NotificationListener: Send + Sync {
    /// A balloon is now showing (or showing again).
    fn notification_shown(&self, _source: &NotificationSource) {}
}

pub const NOTIFICATION_TOPIC: Topic<dyn NotificationListener> = Topic::new("Notifications");

#[derive(Default)]
struct State {
    update: bool,
    /// `(plugin id, notification)`, newest last.
    plugins: Vec<(String, Notification)>,
}

pub struct Notifications {
    bus: MessageBus,
    state: Mutex<State>,
}

impl Notifications {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: Mutex::default(),
        }
    }

    /// Whether the update balloon is showing.
    pub fn update_balloon(&self) -> bool {
        self.state.lock().update
    }

    pub fn show_update_balloon(&self) {
        self.state.lock().update = true;
        self.announce(NotificationSource::Update);
    }

    pub fn dismiss_update_balloon(&self) {
        self.state.lock().update = false;
    }

    /// Adds a plugin balloon.
    pub fn notify_plugin(&self, plugin: &str, notification: Notification) {
        {
            let mut state = self.state.lock();
            state.plugins.push((plugin.to_owned(), notification));
            let excess = state.plugins.len().saturating_sub(MAX_PLUGIN_NOTIFICATIONS);
            state.plugins.drain(..excess);
        }
        self.announce(NotificationSource::Plugin(plugin.to_owned()));
    }

    pub fn plugin_notification_count(&self) -> usize {
        self.state.lock().plugins.len()
    }

    /// The newest `limit` plugin balloons, oldest first, each with the index that
    /// [`Notifications::dismiss_plugin_notification`] takes.
    pub fn recent_plugin_notifications(&self, limit: usize) -> Vec<(usize, String, Notification)> {
        let state = self.state.lock();
        let start = state.plugins.len().saturating_sub(limit);
        state.plugins[start..]
            .iter()
            .enumerate()
            .map(|(offset, (plugin, notification))| {
                (start + offset, plugin.clone(), notification.clone())
            })
            .collect()
    }

    pub fn dismiss_plugin_notification(&self, index: usize) {
        let mut state = self.state.lock();
        if index < state.plugins.len() {
            state.plugins.remove(index);
        }
    }

    fn announce(&self, source: NotificationSource) {
        self.bus
            .publish(NOTIFICATION_TOPIC, |l| l.notification_shown(&source));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager};
    use std::sync::Arc;

    #[derive(Default)]
    struct Heard(Mutex<Vec<NotificationSource>>);

    impl NotificationListener for Heard {
        fn notification_shown(&self, source: &NotificationSource) {
            self.0.lock().push(source.clone());
        }
    }

    fn service() -> (Application, Arc<Heard>, Notifications) {
        let app = Application::new();
        let heard = Arc::new(Heard::default());
        app.message_bus()
            .subscribe(app.disposable(), NOTIFICATION_TOPIC, heard.clone())
            .unwrap();
        let service = Notifications::new(app.message_bus().clone());
        (app, heard, service)
    }

    #[test]
    fn the_update_balloon_shows_until_dismissed_and_is_announced() {
        let (_app, heard, notifications) = service();
        assert!(!notifications.update_balloon());
        notifications.show_update_balloon();
        assert!(notifications.update_balloon());
        notifications.dismiss_update_balloon();
        assert!(!notifications.update_balloon());
        assert_eq!(*heard.0.lock(), [NotificationSource::Update]);
    }

    #[test]
    fn plugin_balloons_keep_the_newest_twenty_and_dismiss_by_index() {
        let (_app, heard, notifications) = service();
        for n in 0..25 {
            notifications.notify_plugin("p", Notification::info(format!("n{n}"), ""));
        }
        assert_eq!(notifications.plugin_notification_count(), 20);
        assert_eq!(heard.0.lock().len(), 25);
        let recent = notifications.recent_plugin_notifications(3);
        let titles: Vec<_> = recent.iter().map(|(_, _, n)| n.title.as_str()).collect();
        assert_eq!(titles, ["n22", "n23", "n24"]);
        assert_eq!(recent[0].0, 17, "indices count within the kept list");
        notifications.dismiss_plugin_notification(19);
        notifications.dismiss_plugin_notification(99);
        assert_eq!(notifications.plugin_notification_count(), 19);
        assert_eq!(
            notifications.recent_plugin_notifications(1)[0].2.title,
            "n23"
        );
    }
}
