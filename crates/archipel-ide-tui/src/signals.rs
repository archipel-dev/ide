//! SIGTERM, SIGINT and SIGHUP end the terminal IDE like Quit does (QA bug 21 of the TUI list): the frame loop
//! sees the request, saves the documents and leaves through the normal path, which gives the
//! terminal back (alternate screen, mouse tracking, raw mode, bracketed paste, kitty flags).
//! The handler only sets a flag; everything else happens on the main thread.

use std::sync::atomic::{AtomicBool, Ordering};

static STOP: AtomicBool = AtomicBool::new(false);

/// Whether a termination signal arrived (or [`request_stop`] was called).
pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// Asks the IDE to quit, as a signal does.
pub fn request_stop() {
    STOP.store(true, Ordering::SeqCst);
}

/// Forgets a request (tests).
pub fn clear() {
    STOP.store(false, Ordering::SeqCst);
}

/// Installs the handlers; a no-op where there are no such signals.
pub fn install() {
    #[cfg(unix)]
    {
        extern "C" fn handler(_: libc::c_int) {
            STOP.store(true, Ordering::SeqCst);
        }
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            // SAFETY: the handler only stores into an atomic, which is async-signal-safe.
            unsafe {
                libc::signal(signal, handler as *const () as libc::sighandler_t);
            }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_signal_sets_the_stop_request() {
        install();
        clear();
        assert!(!stop_requested());
        // SAFETY: raising a signal whose handler was installed above.
        unsafe { libc::raise(libc::SIGTERM) };
        assert!(stop_requested());
        clear();
    }
}
