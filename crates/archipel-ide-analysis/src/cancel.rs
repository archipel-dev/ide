//! Cooperative cancellation. Background work receives a [`CancelToken`] and
//! returns `Err(Cancelled)` soon after the host revokes it (an edit, a newer
//! job with the same coalescing key). `Cancelled` is a normal value: it is
//! never shown to the user and never a panic.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The computation was revoked; discard its partial result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("cancelled")]
pub struct Cancelled;

/// Shared cancellation flag; clones observe the same flag. A child token is
/// cancelled when it or any ancestor is.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
    parent: Option<Arc<CancelToken>>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// A token cancelled together with `self` (and on its own).
    pub fn child(&self) -> Self {
        Self {
            flag: Arc::default(),
            parent: Some(Arc::new(self.clone())),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
            || self
                .parent
                .as_ref()
                .is_some_and(|parent| parent.is_cancelled())
    }

    /// `Err(Cancelled)` once cancelled. Use `?` at cancellation points.
    pub fn check(&self) -> Result<(), Cancelled> {
        if self.is_cancelled() {
            Err(Cancelled)
        } else {
            Ok(())
        }
    }

    /// A counter that polls the flag every [`CHECK_INTERVAL`] ticks, for hot
    /// loops (tree walks, per-line scans).
    pub fn checkpoint(&self) -> Checkpoint<'_> {
        Checkpoint {
            token: self,
            ticks: 0,
        }
    }
}

/// Ticks between two flag polls of a [`Checkpoint`]. Work observing a revoke
/// stops within this many ticks (the contract is "within 256").
pub const CHECK_INTERVAL: u32 = 64;

/// See [`CancelToken::checkpoint`].
pub struct Checkpoint<'a> {
    token: &'a CancelToken,
    ticks: u32,
}

impl Checkpoint<'_> {
    /// Counts one unit of work; every [`CHECK_INTERVAL`]th call polls.
    #[inline]
    pub fn tick(&mut self) -> Result<(), Cancelled> {
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks.is_multiple_of(CHECK_INTERVAL) {
            self.token.check()
        } else {
            Ok(())
        }
    }

    pub fn ticks(&self) -> u32 {
        self.ticks
    }
}

/// Visits `node`'s subtree in pre-order, polling `cancel` every
/// [`CHECK_INTERVAL`] nodes.
pub fn walk<'s>(
    node: archipel_ide_language::SyntaxNode<'s>,
    cancel: &CancelToken,
    mut visit: impl FnMut(archipel_ide_language::SyntaxNode<'s>),
) -> Result<(), Cancelled> {
    let mut checkpoint = cancel.checkpoint();
    for node in node.preorder() {
        checkpoint.tick()?;
        visit(node);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn children_follow_parents() {
        let parent = CancelToken::new();
        let child = parent.child();
        let other = child.clone();
        assert!(child.check().is_ok());
        parent.cancel();
        assert_eq!(other.check(), Err(Cancelled));
        let lone = CancelToken::new();
        lone.child().cancel();
        assert!(!lone.is_cancelled());
    }

    #[test]
    fn checkpoints_stop_within_the_interval() {
        let token = CancelToken::new();
        let mut checkpoint = token.checkpoint();
        for _ in 0..1000 {
            checkpoint.tick().expect("not cancelled");
        }
        token.cancel();
        let mut ticks = 0;
        while checkpoint.tick().is_ok() {
            ticks += 1;
        }
        assert!(ticks < CHECK_INTERVAL as usize && CHECK_INTERVAL <= 256);
    }
}
