//! Embeds build metadata for `--version` and Help → About (`build_info.rs`).
//!
//! Commit and date come from git when the source is a checkout, else from
//! `ARCHIPEL_BUILD_COMMIT` / `GITHUB_SHA` and `SOURCE_DATE_EPOCH`. The date is
//! the commit date, so rebuilding the same commit gives the same output.

use std::path::Path;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (output.status.success() && !text.is_empty()).then_some(text)
}

/// `YYYY-MM-DD` for a Unix timestamp (civil-from-days, proleptic Gregorian).
fn date_from_epoch(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn main() {
    for var in [
        "ARCHIPEL_BUILD_COMMIT",
        "JETTHING_BUILD_COMMIT", // compatibility: name before the rename
        "GITHUB_SHA",
        "SOURCE_DATE_EPOCH",
    ] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    // Rebuild when HEAD moves (checkouts, commits).
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        let head = Path::new(&git_dir).join("HEAD");
        println!("cargo:rerun-if-changed={}", head.display());
        if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!(
                "cargo:rerun-if-changed={}",
                Path::new(&git_dir).join(reference).display()
            );
        }
    }
    // `JETTHING_BUILD_COMMIT` is the name before the rename to Archipel IDE; still read.
    let commit = std::env::var("ARCHIPEL_BUILD_COMMIT")
        .or_else(|_| std::env::var("JETTHING_BUILD_COMMIT"))
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| git(&["rev-parse", "HEAD"]))
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .map(|sha| sha.chars().take(12).collect::<String>())
        .unwrap_or_else(|| "unknown".into());
    let date = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|epoch| epoch.trim().parse::<i64>().ok())
        .map(date_from_epoch)
        .or_else(|| git(&["log", "-1", "--format=%cs"]))
        .unwrap_or_else(|| "unknown".into());
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=ARCHIPEL_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=ARCHIPEL_BUILD_DATE={date}");
    println!("cargo:rustc-env=ARCHIPEL_BUILD_TARGET={target}");
    println!("cargo:rustc-env=ARCHIPEL_BUILD_PROFILE={profile}");
}
