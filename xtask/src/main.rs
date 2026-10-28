mod changelog;
mod design_lint;
mod dist;
mod editor_cases;
mod java_cases;
mod manifests;
mod parity;
mod scenario;
mod scenes;
mod search_cases;
mod spec_coverage;
mod ux_smoke;

use anyhow::{Context, Result, anyhow, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const USAGE: &str = "usage: cargo xtask parity [--check] | dist [--installer] [--target <triple>] | changelog [--from REF] [--to REF] [--version X.Y.Z] [--output FILE] | manifests --dir DIR [--version X.Y.Z] | scenes [--only 01,05] [--out DIR] [--release] | scenario [--only NAME,…] [--report FILE] [--install-toolchains] [--list] [--strict] [--release] | ux-smoke [--only NAME,…] [--out DIR] [--release] | spec-coverage [--check] [--out FILE] | design-lint [--update] | editor-cases [--only ED-TYPE] [--verbose] | java-cases [--only JAVA-CMP] [--verbose] | search-cases [--only SRCH-MAT] [--verbose] | pack-plugin <dir> [output] | build-wasm-example";

fn workspace_root() -> PathBuf {
    // Prefer the current directory when it holds the inventory (normal `cargo run`
    // from the workspace root); fall back to the xtask manifest's parent.
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("docs/parity").is_dir() {
        cwd
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("parity") => {
            let mut check = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--check" => check = true,
                    other => bail!("unknown argument `{other}`; {USAGE}"),
                }
            }
            parity(check)
        }
        Some("dist") => dist::dist(dist::Options::parse(args.into_iter().skip(1))?),
        Some("changelog") => changelog::run(changelog::Options::parse(args.into_iter().skip(1))?),
        Some("manifests") => manifests::run(manifests::Options::parse(args.into_iter().skip(1))?),
        Some("scenes") => scenes::run(args.into_iter().skip(1)),
        Some("scenario") => scenario::run(args.into_iter().skip(1)),
        Some("ux-smoke") => ux_smoke::run(args.into_iter().skip(1)),
        Some("spec-coverage") => spec_coverage::run(&workspace_root(), args.into_iter().skip(1)),
        Some("design-lint") => design_lint::run(args.into_iter().skip(1)),
        Some("editor-cases") => editor_cases::run(&workspace_root(), args.into_iter().skip(1)),
        Some("java-cases") => java_cases::run(&workspace_root(), args.into_iter().skip(1)),
        Some("search-cases") => search_cases::run(&workspace_root(), args.into_iter().skip(1)),
        Some("build-wasm-example") => build_wasm_example(),
        Some("pack-plugin") => {
            let dir = args
                .get(1)
                .context("usage: cargo xtask pack-plugin <plugin dir> [output]")?;
            pack_plugin(Path::new(dir), args.get(2).map(Path::new))
        }
        _ => bail!(USAGE),
    }
}

fn parity(check: bool) -> Result<()> {
    match parity::run(&workspace_root(), check).map_err(|e| anyhow!("{e}"))? {
        parity::Outcome::Valid { rows } => println!("parity tables valid: {rows} rows"),
        parity::Outcome::UpToDate => println!("parity dashboard and README are up to date"),
        parity::Outcome::Written(paths) => {
            for path in paths {
                println!("wrote {}", path.display());
            }
        }
    }
    Ok(())
}

/// Rebuilds `plugins/hello-wasm/hello_wasm.wasm` (needs the `wasm32-wasip2` target).
fn build_wasm_example() -> Result<()> {
    let dir = Path::new("plugins/hello-wasm");
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--release", "--target", "wasm32-wasip2"])
        .current_dir(dir)
        .status()
        .context("run cargo for the wasm example")?;
    if !status.success() {
        bail!(
            "building the wasm example failed (install the target: rustup target add wasm32-wasip2)"
        );
    }
    fs::copy(
        dir.join("target/wasm32-wasip2/release/archipel_ide_plugin_hello_wasm.wasm"),
        dir.join("hello_wasm.wasm"),
    )?;
    println!("{}", dir.join("hello_wasm.wasm").display());
    Ok(())
}

/// Packs a plugin directory into `dist/plugins/<id>-<version>.archipelplugin`.
fn pack_plugin(dir: &Path, output: Option<&Path>) -> Result<()> {
    let manifest_text = fs::read_to_string(dir.join(archipel_ide_plugins::MANIFEST_FILE))
        .with_context(|| {
            format!(
                "{} has no {}",
                dir.display(),
                archipel_ide_plugins::MANIFEST_FILE
            )
        })?;
    let manifest = archipel_ide_plugins::PluginManifest::parse(&manifest_text)?;
    let default = Path::new("dist/plugins").join(format!(
        "{}-{}.{}",
        manifest.id,
        manifest.version,
        archipel_ide_plugins::archive::ARCHIVE_EXTENSION
    ));
    let output = output.unwrap_or(&default);
    archipel_ide_plugins::archive::pack(dir, output)?;
    // Installing validates the archive the same way the IDE will.
    archipel_ide_plugins::archive::read_manifest(output)?;
    println!("{}", output.display());
    Ok(())
}
