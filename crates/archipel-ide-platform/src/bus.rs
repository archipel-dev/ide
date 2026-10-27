//! `MessageBus` with typed `Topic`s (IntelliJ: `MessageBus`, `Topic`, `MessageBusConnection`).
//!
//! A topic names an event interface; the listener type is a trait object, e.g.
//! `Topic<dyn ProjectManagerListener>`. Subscribers connect under a [`DisposableId`], so the
//! subscription ends when its owner is disposed. Publishing is synchronous: every subscriber
//! has run when [`MessageBus::publish`] returns, in subscription order, on the calling thread.
//! Background threads do not publish directly; they [`post`](BusSender::post) a message that
//! the UI thread delivers with [`MessageBus::deliver_pending`] (typically once per frame).
//!
//! Differences from IntelliJ: there is no generated proxy, so the publisher passes a closure
//! that calls the listener method; there is no broadcast to child buses (subscribe on the bus
//! you care about); a nested publish runs immediately instead of being queued.

use crate::disposer::{DisposableId, Disposed, Disposer};
use parking_lot::Mutex;
use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

/// A named channel for one listener interface `L` (usually `dyn Trait`).
///
/// Topics are plain values; declare them as `pub const`. Two topics are the same channel when
/// both the listener type and the name match.
pub struct Topic<L: ?Sized + 'static> {
    name: &'static str,
    marker: PhantomData<fn() -> Box<L>>,
}

impl<L: ?Sized + 'static> Topic<L> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    fn key(&self) -> TopicKey {
        TopicKey {
            listener: TypeId::of::<L>(),
            name: self.name,
        }
    }
}

impl<L: ?Sized + 'static> Clone for Topic<L> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<L: ?Sized + 'static> Copy for Topic<L> {}

impl<L: ?Sized + 'static> fmt::Debug for Topic<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Topic({})", self.name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TopicKey {
    listener: TypeId,
    name: &'static str,
}

struct Subscription {
    connection: DisposableId,
    /// An `Arc<L>` for the topic's listener type.
    listener: Arc<dyn Any + Send + Sync>,
}

type Message = Box<dyn FnOnce(&MessageBus) + Send>;
type Waker = Arc<dyn Fn() + Send + Sync>;

struct BusInner {
    name: String,
    disposer: Disposer,
    subscribers: Mutex<HashMap<TopicKey, Vec<Subscription>>>,
    queue: Mutex<VecDeque<Message>>,
    waker: Mutex<Option<Waker>>,
    failures: Mutex<Vec<String>>,
}

/// Delivers events to the subscribers of a topic. Cloning gives another handle to the same bus.
#[derive(Clone)]
pub struct MessageBus {
    inner: Arc<BusInner>,
}

impl fmt::Debug for MessageBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MessageBus({})", self.inner.name)
    }
}

impl MessageBus {
    /// A bus whose connections live in `disposer`. `name` only appears in diagnostics.
    pub fn new(disposer: Disposer, name: &str) -> Self {
        Self {
            inner: Arc::new(BusInner {
                name: name.to_owned(),
                disposer,
                subscribers: Mutex::default(),
                queue: Mutex::default(),
                waker: Mutex::default(),
                failures: Mutex::default(),
            }),
        }
    }

    /// Opens a connection that ends when `parent` is disposed (IntelliJ
    /// `messageBus.connect(parentDisposable)`).
    pub fn connect(&self, parent: DisposableId) -> Result<MessageBusConnection, Disposed> {
        let node = self
            .inner
            .disposer
            .new_child(parent, "message bus connection")?;
        let bus = Arc::downgrade(&self.inner);
        self.inner
            .disposer
            .register_fn(node, "unsubscribe", move || {
                if let Some(bus) = bus.upgrade() {
                    let mut topics = bus.subscribers.lock();
                    for subscriptions in topics.values_mut() {
                        subscriptions.retain(|s| s.connection != node);
                    }
                    topics.retain(|_, subscriptions| !subscriptions.is_empty());
                }
            })?;
        Ok(MessageBusConnection {
            bus: self.clone(),
            node,
        })
    }

    /// Subscribes `listener` until `parent` is disposed.
    pub fn subscribe<L: ?Sized + Send + Sync + 'static>(
        &self,
        parent: DisposableId,
        topic: Topic<L>,
        listener: Arc<L>,
    ) -> Result<(), Disposed> {
        self.connect(parent)?.subscribe(topic, listener);
        Ok(())
    }

    /// Calls `invoke` for every subscriber of `topic`, in subscription order, before
    /// returning. Subscribers added or removed while publishing take effect for the next
    /// publish. A panicking subscriber is recorded (see [`MessageBus::take_failures`]) and
    /// does not stop the others.
    pub fn publish<L: ?Sized + Send + Sync + 'static>(&self, topic: Topic<L>, invoke: impl Fn(&L)) {
        let listeners: Vec<Arc<L>> = self
            .inner
            .subscribers
            .lock()
            .get(&topic.key())
            .map(|subs| {
                subs.iter()
                    .filter_map(|s| s.listener.downcast_ref::<Arc<L>>().cloned())
                    .collect()
            })
            .unwrap_or_default();
        for listener in listeners {
            if catch_unwind(AssertUnwindSafe(|| invoke(&listener))).is_err() {
                let failure = format!(
                    "a listener of `{}` panicked on bus `{}`",
                    topic.name, self.inner.name
                );
                eprintln!("archipel-ide-platform: {failure}");
                self.inner.failures.lock().push(failure);
            }
        }
    }

    /// How many listeners are subscribed to `topic`.
    pub fn subscriber_count<L: ?Sized + 'static>(&self, topic: Topic<L>) -> usize {
        self.inner
            .subscribers
            .lock()
            .get(&topic.key())
            .map_or(0, Vec::len)
    }

    /// A handle that background threads use to queue events for the UI thread.
    pub fn sender(&self) -> BusSender {
        BusSender {
            inner: self.inner.clone(),
        }
    }

    /// Registers what wakes the UI when a message was posted (e.g. a repaint request).
    pub fn set_waker(&self, waker: impl Fn() + Send + Sync + 'static) {
        *self.inner.waker.lock() = Some(Arc::new(waker));
    }

    /// Delivers the messages posted so far (see [`BusSender::post`]) in posting order and
    /// returns how many. Messages posted by listeners during delivery wait for the next call.
    /// Call on the UI thread.
    pub fn deliver_pending(&self) -> usize {
        let batch: Vec<Message> = self.inner.queue.lock().drain(..).collect();
        let delivered = batch.len();
        for message in batch {
            message(self);
        }
        delivered
    }

    /// How many posted messages wait for [`MessageBus::deliver_pending`].
    pub fn pending(&self) -> usize {
        self.inner.queue.lock().len()
    }

    /// Descriptions of listener panics since the last call.
    pub fn take_failures(&self) -> Vec<String> {
        std::mem::take(&mut *self.inner.failures.lock())
    }
}

/// A set of subscriptions that ends together (IntelliJ `MessageBusConnection`).
pub struct MessageBusConnection {
    bus: MessageBus,
    node: DisposableId,
}

impl MessageBusConnection {
    pub fn subscribe<L: ?Sized + Send + Sync + 'static>(&self, topic: Topic<L>, listener: Arc<L>) {
        self.bus
            .inner
            .subscribers
            .lock()
            .entry(topic.key())
            .or_default()
            .push(Subscription {
                connection: self.node,
                listener: Arc::new(listener),
            });
    }

    /// Ends the subscriptions now instead of with the parent.
    pub fn disconnect(self) {
        self.bus.inner.disposer.dispose(self.node);
    }
}

/// Posts events from any thread; the UI thread delivers them.
#[derive(Clone)]
pub struct BusSender {
    inner: Arc<BusInner>,
}

impl BusSender {
    /// Queues an event of `topic`. It is published by the next
    /// [`MessageBus::deliver_pending`], and the bus's waker is called so the UI gets there.
    pub fn post<L: ?Sized + Send + Sync + 'static>(
        &self,
        topic: Topic<L>,
        invoke: impl Fn(&L) + Send + 'static,
    ) {
        self.inner
            .queue
            .lock()
            .push_back(Box::new(move |bus: &MessageBus| bus.publish(topic, invoke)));
        let waker = self.inner.waker.lock().clone();
        if let Some(waker) = waker {
            waker();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    trait Greeting: Send + Sync {
        fn hello(&self, who: &str);
    }

    const GREETING: Topic<dyn Greeting> = Topic::new("greeting");
    const OTHER_GREETING: Topic<dyn Greeting> = Topic::new("other greeting");

    #[derive(Default)]
    struct Log(Mutex<Vec<String>>);

    struct Tagged(&'static str, Arc<Log>);

    impl Greeting for Tagged {
        fn hello(&self, who: &str) {
            self.1.0.lock().push(format!("{} {who}", self.0));
        }
    }

    fn setup() -> (Disposer, MessageBus, DisposableId) {
        let disposer = Disposer::new();
        let owner = disposer.new_disposable("owner");
        (disposer.clone(), MessageBus::new(disposer, "test"), owner)
    }

    #[test]
    fn publish_reaches_subscribers_in_subscription_order() {
        let (_, bus, owner) = setup();
        let log = Arc::new(Log::default());
        bus.subscribe(owner, GREETING, Arc::new(Tagged("first", log.clone())))
            .unwrap();
        bus.subscribe(owner, GREETING, Arc::new(Tagged("second", log.clone())))
            .unwrap();
        bus.publish(GREETING, |l| l.hello("world"));
        assert_eq!(*log.0.lock(), ["first world", "second world"]);
    }

    #[test]
    fn topics_with_the_same_listener_type_are_separate_channels() {
        let (_, bus, owner) = setup();
        let log = Arc::new(Log::default());
        bus.subscribe(
            owner,
            OTHER_GREETING,
            Arc::new(Tagged("other", log.clone())),
        )
        .unwrap();
        bus.publish(GREETING, |l| l.hello("x"));
        assert!(log.0.lock().is_empty());
        bus.publish(OTHER_GREETING, |l| l.hello("y"));
        assert_eq!(*log.0.lock(), ["other y"]);
    }

    #[test]
    fn disposing_the_owner_ends_the_subscription() {
        let (disposer, bus, owner) = setup();
        let log = Arc::new(Log::default());
        bus.subscribe(owner, GREETING, Arc::new(Tagged("a", log.clone())))
            .unwrap();
        assert_eq!(bus.subscriber_count(GREETING), 1);
        disposer.dispose(owner);
        assert_eq!(bus.subscriber_count(GREETING), 0);
        bus.publish(GREETING, |l| l.hello("late"));
        assert!(log.0.lock().is_empty());
        assert!(
            bus.subscribe(owner, GREETING, Arc::new(Tagged("b", log)))
                .is_err()
        );
    }

    #[test]
    fn a_connection_groups_subscriptions_and_can_disconnect_early() {
        let (_, bus, owner) = setup();
        let log = Arc::new(Log::default());
        let kept = Arc::new(Log::default());
        bus.subscribe(owner, GREETING, Arc::new(Tagged("kept", kept.clone())))
            .unwrap();
        let connection = bus.connect(owner).unwrap();
        connection.subscribe(GREETING, Arc::new(Tagged("a", log.clone())));
        connection.subscribe(OTHER_GREETING, Arc::new(Tagged("b", log.clone())));
        bus.publish(GREETING, |l| l.hello("1"));
        connection.disconnect();
        bus.publish(GREETING, |l| l.hello("2"));
        bus.publish(OTHER_GREETING, |l| l.hello("3"));
        assert_eq!(*log.0.lock(), ["a 1"]);
        assert_eq!(*kept.0.lock(), ["kept 1", "kept 2"]);
    }

    #[test]
    fn a_panicking_listener_is_recorded_and_the_rest_still_run() {
        struct Boom;
        impl Greeting for Boom {
            fn hello(&self, _: &str) {
                panic!("boom");
            }
        }
        let (_, bus, owner) = setup();
        let log = Arc::new(Log::default());
        bus.subscribe(owner, GREETING, Arc::new(Boom)).unwrap();
        bus.subscribe(owner, GREETING, Arc::new(Tagged("ok", log.clone())))
            .unwrap();
        bus.publish(GREETING, |l| l.hello("x"));
        assert_eq!(*log.0.lock(), ["ok x"]);
        assert_eq!(bus.take_failures().len(), 1);
        assert!(bus.take_failures().is_empty());
    }

    #[test]
    fn a_listener_may_subscribe_and_publish_while_handling() {
        struct Reentrant(MessageBus, DisposableId, Arc<AtomicUsize>);
        impl Greeting for Reentrant {
            fn hello(&self, who: &str) {
                self.2.fetch_add(1, Ordering::SeqCst);
                if who == "outer" {
                    self.0.publish(GREETING, |l| l.hello("inner"));
                    let log = Arc::new(Log::default());
                    self.0
                        .subscribe(self.1, GREETING, Arc::new(Tagged("new", log)))
                        .unwrap();
                }
            }
        }
        let (_, bus, owner) = setup();
        let calls = Arc::new(AtomicUsize::new(0));
        bus.subscribe(
            owner,
            GREETING,
            Arc::new(Reentrant(bus.clone(), owner, calls.clone())),
        )
        .unwrap();
        bus.publish(GREETING, |l| l.hello("outer"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(bus.subscriber_count(GREETING), 2);
    }

    #[test]
    fn background_threads_post_and_the_ui_thread_delivers() {
        let (_, bus, owner) = setup();
        let log = Arc::new(Log::default());
        bus.subscribe(owner, GREETING, Arc::new(Tagged("ui", log.clone())))
            .unwrap();
        let woken = Arc::new(AtomicUsize::new(0));
        let counter = woken.clone();
        bus.set_waker(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        let sender = bus.sender();
        let thread = std::thread::spawn(move || {
            for who in ["a", "b"] {
                sender.post(GREETING, move |l| l.hello(who));
            }
        });
        thread.join().unwrap();
        assert!(
            log.0.lock().is_empty(),
            "nothing is delivered off the UI thread"
        );
        assert_eq!(bus.pending(), 2);
        assert_eq!(woken.load(Ordering::SeqCst), 2);
        assert_eq!(bus.deliver_pending(), 2);
        assert_eq!(*log.0.lock(), ["ui a", "ui b"]);
        assert_eq!(bus.deliver_pending(), 0);
    }

    #[test]
    fn messages_posted_during_delivery_wait_for_the_next_call() {
        struct Chain(BusSender);
        impl Greeting for Chain {
            fn hello(&self, who: &str) {
                if who == "first" {
                    self.0.post(GREETING, |l| l.hello("second"));
                }
            }
        }
        let (_, bus, owner) = setup();
        let log = Arc::new(Log::default());
        bus.subscribe(owner, GREETING, Arc::new(Chain(bus.sender())))
            .unwrap();
        bus.subscribe(owner, GREETING, Arc::new(Tagged("t", log.clone())))
            .unwrap();
        bus.sender().post(GREETING, |l| l.hello("first"));
        assert_eq!(bus.deliver_pending(), 1);
        assert_eq!(bus.pending(), 1);
        assert_eq!(bus.deliver_pending(), 1);
        assert_eq!(*log.0.lock(), ["t first", "t second"]);
    }
}
