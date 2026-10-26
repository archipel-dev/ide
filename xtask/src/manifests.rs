//! `cargo xtask manifests`: package-manager manifests for a release.
//!
//! ```text
//! cargo xtask manifests --dir <artifacts> [--version X.Y.Z] [--repository owner/repo] [--out DIR]
//! ```
//!
//! Reads the `<artifact>.sha256` files that `cargo xtask dist` wrote (searched
//! recursively under `--dir`, e.g. all downloaded CI artifacts) and renders:
//!
//! - Homebrew cask `homebrew/archipel-ide.rb` (both macOS DMGs),
//! - winget `winget/manifests/a/ArchipelDev/ArchipelIDE/<version>/` (MSI, schema 1.6.0),
//! - Scoop `scoop/archipel-ide.json` (portable Windows zip, with `checkver`/`autoupdate`).
//!
//! Nothing is published; `docs/release.md` describes submitting them.

use anyhow::{Context, Result, bail};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const PACKAGE_ID: &str = "ArchipelDev.ArchipelIDE";
const DESCRIPTION: &str = "Native Rust IDE platform";
const WINGET_SCHEMA: &str = "1.6.0";

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub dir: PathBuf,
    pub version: String,
    pub repository: String,
    pub out: Option<PathBuf>,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Options> {
        let mut dir = None;
        let mut version = env!("CARGO_PKG_VERSION").to_owned();
        let mut repository = "archipel-dev/ide".to_owned();
        let mut out = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--dir" => dir = Some(PathBuf::from(value()?)),
                "--version" => version = value()?,
                "--repository" => repository = value()?,
                "--out" => out = Some(PathBuf::from(value()?)),
                other => bail!("unknown manifests option `{other}`"),
            }
        }
        Ok(Options {
            dir: dir.context("--dir <artifacts directory> is required")?,
            version,
            repository,
            out,
        })
    }
}

/// Release facts the manifests need: download base URL and per-file digests.
pub struct Release {
    pub version: String,
    pub repository: String,
    /// File name → lowercase hex SHA-256.
    pub digests: BTreeMap<String, String>,
}

impl Release {
    fn homepage(&self) -> String {
        format!("https://github.com/{}", self.repository)
    }

    fn url(&self, file: &str) -> String {
        format!(
            "https://github.com/{}/releases/download/v{}/{file}",
            self.repository, self.version
        )
    }

    fn digest(&self, file: &str) -> Result<&str> {
        self.digests
            .get(file)
            .map(String::as_str)
            .with_context(|| format!("no {file}.sha256 among the artifacts"))
    }
}

/// Reads every `*.sha256` (sha256sum format) below `dir`.
pub fn collect_digests(dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut digests = BTreeMap::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(current) = pending.pop() {
        for entry in
            fs::read_dir(&current).with_context(|| format!("read {}", current.display()))?
        {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(artifact) = name.strip_suffix(".sha256") else {
                continue;
            };
            let text = fs::read_to_string(&path)?;
            let digest = text
                .split_whitespace()
                .next()
                .filter(|d| d.len() == 64 && d.chars().all(|c| c.is_ascii_hexdigit()))
                .with_context(|| format!("{} is not a sha256 file", path.display()))?;
            digests.insert(artifact.to_owned(), digest.to_ascii_lowercase());
        }
    }
    Ok(digests)
}

pub fn homebrew_cask(release: &Release) -> Result<String> {
    let version = &release.version;
    let arm = release.digest(&format!("archipel-ide-{version}-macos-aarch64.dmg"))?;
    let intel = release.digest(&format!("archipel-ide-{version}-macos-x86_64.dmg"))?;
    let repository = &release.repository;
    Ok(format!(
        r##"cask "archipel-ide" do
  arch arm: "aarch64", intel: "x86_64"

  version "{version}"
  sha256 arm:   "{arm}",
         intel: "{intel}"

  url "https://github.com/{repository}/releases/download/v#{{version}}/archipel-ide-#{{version}}-macos-#{{arch}}.dmg"
  name "Archipel IDE"
  desc "{DESCRIPTION}"
  homepage "{homepage}"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :big_sur"

  app "Archipel IDE.app"
  binary "#{{appdir}}/Archipel IDE.app/Contents/MacOS/archipel-ide"
  binary "#{{appdir}}/Archipel IDE.app/Contents/MacOS/archipel-cli"

  zap trash: "~/Library/Logs/Archipel"
end
"##,
        homepage = release.homepage(),
    ))
}

/// The three winget files (version, installer, default locale) by file name.
pub fn winget(release: &Release) -> Result<Vec<(String, String)>> {
    let version = &release.version;
    let msi = format!("archipel-ide-{version}-windows-x86_64.msi");
    let sha = release.digest(&msi)?.to_ascii_uppercase();
    let header = |kind: &str| {
        format!(
            "# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.{WINGET_SCHEMA}.schema.json\n\
             PackageIdentifier: {PACKAGE_ID}\n\
             PackageVersion: {version}\n"
        )
    };
    let footer = |kind: &str| format!("ManifestType: {kind}\nManifestVersion: {WINGET_SCHEMA}\n");
    let version_file = format!(
        "{}DefaultLocale: en-US\n{}",
        header("version"),
        footer("version")
    );
    let installer = format!(
        "{}InstallerType: wix\n\
         Scope: machine\n\
         UpgradeBehavior: install\n\
         Installers:\n\
         - Architecture: x64\n  \
           InstallerUrl: {url}\n  \
           InstallerSha256: {sha}\n\
         {}",
        header("installer"),
        footer("installer"),
        url = release.url(&msi),
    );
    let homepage = release.homepage();
    let locale = format!(
        "{}PackageLocale: en-US\n\
         Publisher: Archipel IDE contributors\n\
         PublisherUrl: {homepage}\n\
         PublisherSupportUrl: {homepage}/issues\n\
         PackageName: Archipel IDE\n\
         PackageUrl: {homepage}\n\
         License: Apache-2.0 OR MIT\n\
         ShortDescription: {DESCRIPTION}\n\
         Tags:\n- ide\n- rust\n- editor\n\
         ReleaseNotesUrl: {homepage}/releases/tag/v{version}\n\
         {}",
        header("defaultLocale"),
        footer("defaultLocale"),
    );
    Ok(vec![
        (format!("{PACKAGE_ID}.yaml"), version_file),
        (format!("{PACKAGE_ID}.installer.yaml"), installer),
        (format!("{PACKAGE_ID}.locale.en-US.yaml"), locale),
    ])
}

pub fn scoop(release: &Release) -> Result<String> {
    let version = &release.version;
    let root = format!("archipel-ide-{version}-windows-x86_64");
    let zip = format!("{root}.zip");
    let manifest = json!({
        "version": version,
        "description": DESCRIPTION,
        "homepage": release.homepage(),
        "license": "MIT|Apache-2.0",
        "architecture": {
            "64bit": {
                "url": release.url(&zip),
                "hash": release.digest(&zip)?,
                "extract_dir": root,
            }
        },
        "bin": ["archipel-ide.exe", "archipel-cli.exe"],
        "shortcuts": [["archipel-ide.exe", "Archipel IDE"]],
        "checkver": "github",
        "autoupdate": {
            "architecture": {
                "64bit": {
                    "url": format!(
                        "https://github.com/{}/releases/download/v$version/archipel-ide-$version-windows-x86_64.zip",
                        release.repository
                    ),
                    "extract_dir": "archipel-ide-$version-windows-x86_64",
                    "hash": {"url": "$url.sha256"},
                }
            }
        }
    });
    Ok(format!("{}\n", serde_json::to_string_pretty(&manifest)?))
}

pub fn run(options: Options) -> Result<()> {
    let release = Release {
        digests: collect_digests(&options.dir)?,
        version: options.version,
        repository: options.repository,
    };
    let out = options.out.unwrap_or_else(|| options.dir.join("manifests"));
    let mut written = Vec::new();
    let mut write = |path: PathBuf, text: String| -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
        written.push(path);
        Ok(())
    };
    write(
        out.join("homebrew/archipel-ide.rb"),
        homebrew_cask(&release)?,
    )?;
    let winget_dir = out
        .join("winget/manifests/a/ArchipelDev/ArchipelIDE")
        .join(&release.version);
    for (name, text) in winget(&release)? {
        write(winget_dir.join(name), text)?;
    }
    write(out.join("scoop/archipel-ide.json"), scoop(&release)?)?;
    for path in written {
        println!("{}", path.to_string_lossy().replace('\\', "/"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Release {
        let digest = |c: char| std::iter::repeat_n(c, 64).collect::<String>();
        Release {
            version: "1.2.3".into(),
            repository: "archipel-dev/ide".into(),
            digests: BTreeMap::from([
                ("archipel-ide-1.2.3-macos-aarch64.dmg".into(), digest('a')),
                ("archipel-ide-1.2.3-macos-x86_64.dmg".into(), digest('b')),
                ("archipel-ide-1.2.3-windows-x86_64.msi".into(), digest('c')),
                ("archipel-ide-1.2.3-windows-x86_64.zip".into(), digest('d')),
            ]),
        }
    }

    #[test]
    fn cask_pins_both_dmgs_and_links_the_cli() {
        let cask = homebrew_cask(&release()).unwrap();
        assert!(cask.starts_with("cask \"archipel-ide\" do\n"));
        assert!(cask.contains("version \"1.2.3\""));
        assert!(cask.contains(&format!("sha256 arm:   \"{}\"", "a".repeat(64))));
        assert!(cask.contains(&format!("intel: \"{}\"", "b".repeat(64))));
        assert!(cask.contains(
            "url \"https://github.com/archipel-dev/ide/releases/download/v#{version}/archipel-ide-#{version}-macos-#{arch}.dmg\""
        ));
        assert!(cask.contains("app \"Archipel IDE.app\""));
        assert!(cask.contains("binary \"#{appdir}/Archipel IDE.app/Contents/MacOS/archipel-cli\""));
        assert!(cask.trim_end().ends_with("end"));
    }

    #[test]
    fn winget_manifests_are_consistent() {
        let files = winget(&release()).unwrap();
        let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "ArchipelDev.ArchipelIDE.yaml",
                "ArchipelDev.ArchipelIDE.installer.yaml",
                "ArchipelDev.ArchipelIDE.locale.en-US.yaml"
            ]
        );
        for (_, text) in &files {
            assert!(text.contains("PackageIdentifier: ArchipelDev.ArchipelIDE\n"));
            assert!(text.contains("PackageVersion: 1.2.3\n"));
            assert!(text.ends_with("ManifestVersion: 1.6.0\n"));
            assert!(!text.contains('\t'), "YAML must not contain tabs");
        }
        let installer = &files[1].1;
        assert!(installer.contains("InstallerType: wix\n"));
        assert!(installer.contains(
            "  InstallerUrl: https://github.com/archipel-dev/ide/releases/download/v1.2.3/archipel-ide-1.2.3-windows-x86_64.msi\n"
        ));
        assert!(installer.contains(&format!("  InstallerSha256: {}\n", "C".repeat(64))));
        assert!(files[2].1.contains("ManifestType: defaultLocale\n"));
    }

    #[test]
    fn scoop_manifest_uses_the_portable_zip() {
        let manifest: serde_json::Value =
            serde_json::from_str(&scoop(&release()).unwrap()).unwrap();
        let x64 = &manifest["architecture"]["64bit"];
        assert_eq!(manifest["version"], "1.2.3");
        assert_eq!(x64["hash"], "d".repeat(64));
        assert_eq!(x64["extract_dir"], "archipel-ide-1.2.3-windows-x86_64");
        assert!(
            x64["url"]
                .as_str()
                .unwrap()
                .ends_with("/v1.2.3/archipel-ide-1.2.3-windows-x86_64.zip")
        );
        assert_eq!(manifest["bin"][1], "archipel-cli.exe");
        assert_eq!(
            manifest["autoupdate"]["architecture"]["64bit"]["hash"]["url"],
            "$url.sha256"
        );
    }

    #[test]
    fn missing_artifacts_and_bad_digests_are_reported() {
        let mut partial = release();
        partial
            .digests
            .remove("archipel-ide-1.2.3-macos-x86_64.dmg");
        let error = homebrew_cask(&partial).unwrap_err().to_string();
        assert!(
            error.contains("archipel-ide-1.2.3-macos-x86_64.dmg.sha256"),
            "{error}"
        );

        let dir = std::env::temp_dir().join(format!("xtask-manifests-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(
            dir.join("nested/a.zip.sha256"),
            format!("{}  a.zip\n", "E".repeat(64)),
        )
        .unwrap();
        let digests = collect_digests(&dir).unwrap();
        assert_eq!(digests["a.zip"], "e".repeat(64));
        fs::write(dir.join("bad.sha256"), "nope").unwrap();
        assert!(collect_digests(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn parses_options() {
        let parse = |args: &[&str]| Options::parse(args.iter().map(|a| a.to_string()));
        let options = parse(&["--dir", "artifacts", "--version", "0.0.3"]).unwrap();
        assert_eq!(options.dir, PathBuf::from("artifacts"));
        assert_eq!(options.version, "0.0.3");
        assert!(parse(&[]).is_err());
        assert!(parse(&["--dir"]).is_err());
    }
}
