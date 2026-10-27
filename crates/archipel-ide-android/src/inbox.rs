//! A receiving end of a channel that can be shared between threads.
//!
//! `std::sync::mpsc::Receiver` is `Send` but not `Sync`, so a state that holds one cannot live
//! in a platform service (services are `Arc<T>` with `T: Send + Sync`). An [`Inbox`] wraps it in
//! a mutex; only the owner polls it, so the lock is never contended.

use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// A `Sync` receiver.
pub struct Inbox<T>(Mutex<Receiver<T>>);

impl<T> Inbox<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        Self(Mutex::new(receiver))
    }

    fn receiver(&self) -> MutexGuard<'_, Receiver<T>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Everything that arrived so far, without blocking.
    pub fn try_iter(&self) -> Vec<T> {
        self.receiver().try_iter().collect()
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.receiver().try_recv()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        self.receiver().recv_timeout(timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn an_inbox_is_sync_and_delivers_in_order() {
        fn sync<T: Sync>() {}
        sync::<Inbox<u8>>();
        let (sender, receiver) = mpsc::channel();
        let inbox = Inbox::new(receiver);
        sender.send(1).unwrap();
        sender.send(2).unwrap();
        assert_eq!(inbox.try_iter(), vec![1, 2]);
        assert!(inbox.try_recv().is_err());
        assert!(inbox.recv_timeout(Duration::from_millis(1)).is_err());
    }
}
