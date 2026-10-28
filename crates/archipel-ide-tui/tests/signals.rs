//! SIGTERM and SIGINT end the IDE through the normal quit path, which saves the documents and
//! gives the terminal back (QA bug 21 of the TUI list). This file has one test: the stop request is
//! process-wide.

mod common;

use common::{Project, Tui};

#[cfg(unix)]
#[test]
fn a_termination_signal_saves_the_documents_and_quits() {
    let project = Project::new("signal", &[("a.txt", "alpha\n")]);
    let mut tui = Tui::open(&project.root, &["a.txt"], 100, 24);
    tui.advance(0.1);
    tui.type_text("edited ");
    assert!(!tui.rt.quit);
    archipel_ide_tui::signals::install();
    // SAFETY: raising a signal whose handler was installed above.
    unsafe { libc::raise(libc::SIGTERM) };
    tui.advance(0.3);
    assert!(tui.rt.quit, "the loop leaves");
    assert_eq!(project.read("a.txt"), "edited alpha\n", "edits are saved");
}
