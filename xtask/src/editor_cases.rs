//! `cargo xtask editor-cases`: runs the executable editor cases of `docs/specs/editor/cases` and
//! regenerates `docs/parity/editor-cases.md`. The cases run in the integration test
//! `crates/archipel-ide-core/tests/editor_cases.rs`; this command only sets its environment.
//!
//! * `--only ED-TYPE` runs the cases whose id starts with the text (the report is not written).
//! * `--verbose` prints every failing case with the expected and the actual text.

use anyhow::{Context, Result, bail};
use std::{path::Path, process::Command};

pub fn run(root: &Path, args: impl Iterator<Item = String>) -> Result<()> {
    let mut args = args;
    let mut only = None;
    let mut verbose = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--only" => only = Some(args.next().context("--only needs an id prefix")?),
            "--verbose" => verbose = true,
            other => bail!(
                "unknown argument `{other}`; usage: cargo xtask editor-cases [--only ED-TYPE] [--verbose]"
            ),
        }
    }
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    command.current_dir(root).args([
        "test",
        "-p",
        "archipel-ide-core",
        "--test",
        "editor_cases",
        "--",
        "--nocapture",
    ]);
    match &only {
        Some(prefix) => {
            command.env("EDITOR_CASES_ONLY", prefix);
        }
        None => {
            command.env("EDITOR_CASES_REPORT", "1");
        }
    }
    if verbose {
        command.env("EDITOR_CASES_VERBOSE", "1");
    }
    let status = command.status().context("failed to start cargo")?;
    if !status.success() {
        bail!("the editor cases test failed (a ratchet or a parse error)");
    }
    if only.is_none() {
        println!("wrote docs/parity/editor-cases.md");
    }
    Ok(())
}
