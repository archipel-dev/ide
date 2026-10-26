//! End-to-end provisioning against a local fake download server.

mod support;

use archipel_ide_toolchains::catalog::{IndexedRelease, ReleaseIndex};
use archipel_ide_toolchains::download::{HttpClient, sha256_hex};
use archipel_ide_toolchains::service::{ToolchainEvent, ToolchainService};
use archipel_ide_toolchains::{
    ArchiveKind, Artifact, Catalog, Checksum, ComponentKind, ComponentSpec, IndexProvider,
    LocateSource, Platform, StaticProvider, ToolVersion, ToolchainError, ToolchainSettings,
    ToolchainStore, Toolchains, VersionRequest,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use support::{FakeServer, tool_archive};

fn platform() -> Platform {
    Platform::parse("linux", "x86_64").unwrap()
}

/// Incompressible payload so archives are large enough to cut mid-way.
fn noise(len: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn spec() -> ComponentSpec {
    let mut spec = ComponentSpec::new(
        "demo-ls",
        "Demo LS",
        ComponentKind::LanguageServer,
        "demo-ls",
    );
    spec.home_env = vec!["DEMO_HOME".into()];
    spec
}

fn release(version: &str, url: String, checksum: Checksum) -> IndexedRelease {
    IndexedRelease {
        version: ToolVersion::new(version),
        channels: vec!["stable".into()],
        artifacts: BTreeMap::from([(
            platform().key(),
            Artifact {
                url,
                checksum,
                archive: ArchiveKind::TarGz,
                size: None,
            },
        )]),
        license: None,
    }
}

/// Toolchains with an empty PATH and environment, rooted in `root`.
fn toolchains(catalog: Catalog, root: &Path) -> Toolchains {
    Toolchains::new(catalog, ToolchainStore::new(root))
        .with_platform(platform())
        .with_environment(OsString::new(), BTreeMap::new())
}

fn index_catalog(server: &FakeServer, index: &ReleaseIndex) -> Catalog {
    server.serve("/index.json", serde_json::to_vec(index).unwrap());
    let mut catalog = Catalog::new();
    catalog.register(Arc::new(IndexProvider {
        spec: spec(),
        url: server.url("/index.json"),
    }));
    catalog
}

#[test]
fn resolves_downloads_verifies_installs_and_uninstalls() {
    let server = FakeServer::start();
    let old = tool_archive("demo-1.0.0", "demo-ls", b"old");
    let new = tool_archive("demo-1.2.0", "demo-ls", b"new");
    server.serve("/demo-1.0.0.tar.gz", old.clone());
    server.serve("/demo-1.2.0.tar.gz", new.clone());
    // Checksum from a sha256sum-style file for one, inline digest for the other.
    server.serve(
        "/demo-1.2.0.tar.gz.sha256",
        format!("{}  demo-1.2.0.tar.gz\n", sha256_hex(&new)),
    );
    let index = ReleaseIndex {
        releases: vec![
            release(
                "1.0.0",
                server.url("/demo-1.0.0.tar.gz"),
                Checksum::Sha256(sha256_hex(&old)),
            ),
            release(
                "1.2.0",
                server.url("/demo-1.2.0.tar.gz"),
                Checksum::Sha256Url(server.url("/demo-1.2.0.tar.gz.sha256")),
            ),
        ],
    };
    let dir = tempfile::tempdir().unwrap();
    let toolchains = toolchains(index_catalog(&server, &index), dir.path());
    let http = HttpClient::new();

    assert_eq!(toolchains.missing(&["demo-ls"]), vec!["demo-ls"]);
    let latest = toolchains.resolve("demo-ls", None, &http).unwrap();
    assert_eq!(latest.version.as_str(), "1.2.0");
    let pinned = toolchains
        .resolve("demo-ls", Some(&VersionRequest::parse("1.0")), &http)
        .unwrap();
    assert_eq!(pinned.version.as_str(), "1.0.0");

    let mut phases = Vec::new();
    let installed = toolchains
        .store()
        .install(
            &latest,
            &http,
            &mut |phase| phases.push(phase),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(!phases.is_empty());
    let target = dir.path().join("demo-ls").join("1.2.0");
    assert_eq!(installed.path, target);
    assert_eq!(std::fs::read(target.join("bin/demo-ls")).unwrap(), b"new");
    toolchains
        .store()
        .install(&pinned, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap();

    // Newest managed install is found; the download cache is cleaned up.
    let located = toolchains.locate("demo-ls").unwrap();
    assert_eq!(located.source, LocateSource::Managed);
    assert_eq!(located.version, Some(ToolVersion::new("1.2.0")));
    assert_eq!(located.home, target);
    assert!(toolchains.missing(&["demo-ls"]).is_empty());
    let leftovers: Vec<_> = std::fs::read_dir(dir.path().join(".downloads"))
        .unwrap()
        .collect();
    assert!(leftovers.is_empty());

    // Re-installing an installed version does not download again.
    let before = server.requests("/demo-1.2.0.tar.gz").len();
    toolchains
        .store()
        .install(&latest, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(server.requests("/demo-1.2.0.tar.gz").len(), before);

    toolchains
        .store()
        .uninstall("demo-ls", &ToolVersion::new("1.2.0"))
        .unwrap();
    assert!(!target.exists());
    assert_eq!(
        toolchains.locate("demo-ls").unwrap().version,
        Some(ToolVersion::new("1.0.0"))
    );
    assert!(matches!(
        toolchains
            .store()
            .uninstall("demo-ls", &ToolVersion::new("1.2.0")),
        Err(ToolchainError::NotInstalled { .. })
    ));
}

#[test]
fn checksum_mismatch_installs_nothing_and_discards_the_download() {
    let server = FakeServer::start();
    let archive = tool_archive("demo", "demo-ls", b"payload");
    server.serve("/demo.tar.gz", archive);
    let index = ReleaseIndex {
        releases: vec![release(
            "1.0.0",
            server.url("/demo.tar.gz"),
            Checksum::Sha256("ab".repeat(32)),
        )],
    };
    let dir = tempfile::tempdir().unwrap();
    let toolchains = toolchains(index_catalog(&server, &index), dir.path());
    let http = HttpClient::new();
    let release = toolchains.resolve("demo-ls", None, &http).unwrap();
    let error = toolchains
        .store()
        .install(&release, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap_err();
    assert!(matches!(error, ToolchainError::Checksum { .. }), "{error}");
    assert!(!dir.path().join("demo-ls").exists());
    let partials: Vec<_> = std::fs::read_dir(dir.path().join(".downloads"))
        .unwrap()
        .collect();
    assert!(partials.is_empty(), "mismatching download must be deleted");
}

#[test]
fn interrupted_download_resumes_with_a_range_request() {
    let server = FakeServer::start();
    let mut payload = noise(400_000);
    payload.extend_from_slice(b"end");
    let archive = tool_archive("demo", "demo-ls", &payload);
    let cut = archive.len() / 2;
    server.serve_interrupted("/demo.tar.gz", archive.clone(), cut);
    let index = ReleaseIndex {
        releases: vec![release(
            "1.0.0",
            server.url("/demo.tar.gz"),
            Checksum::Sha256(sha256_hex(&archive)),
        )],
    };
    let dir = tempfile::tempdir().unwrap();
    let toolchains = toolchains(index_catalog(&server, &index), dir.path());
    let http = HttpClient::new();
    let release = toolchains.resolve("demo-ls", None, &http).unwrap();

    let error = toolchains
        .store()
        .install(&release, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap_err();
    assert!(matches!(error, ToolchainError::Network { .. }), "{error}");
    assert!(!dir.path().join("demo-ls").exists());

    let mut resumed_from = None;
    toolchains
        .store()
        .install(
            &release,
            &http,
            &mut |phase| {
                if let archipel_ide_toolchains::store::InstallPhase::Downloading(p) = phase {
                    resumed_from.get_or_insert(p.received);
                }
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    let requests = server.requests("/demo.tar.gz");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].range, None);
    assert_eq!(
        requests[1].range.as_deref(),
        Some(format!("bytes={cut}-").as_str())
    );
    assert_eq!(resumed_from, Some(cut as u64));
    let installed = std::fs::read(dir.path().join("demo-ls/1.0.0/bin/demo-ls")).unwrap();
    assert_eq!(installed, payload);
}

#[test]
fn failed_extraction_leaves_no_partial_install() {
    let server = FakeServer::start();
    let garbage = b"definitely not a gzip stream".to_vec();
    server.serve("/broken.tar.gz", garbage.clone());
    let index = ReleaseIndex {
        releases: vec![release(
            "1.0.0",
            server.url("/broken.tar.gz"),
            Checksum::Sha256(sha256_hex(&garbage)),
        )],
    };
    let dir = tempfile::tempdir().unwrap();
    let toolchains = toolchains(index_catalog(&server, &index), dir.path());
    let http = HttpClient::new();
    let release = toolchains.resolve("demo-ls", None, &http).unwrap();
    let error = toolchains
        .store()
        .install(&release, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap_err();
    assert!(matches!(error, ToolchainError::Archive { .. }), "{error}");
    assert!(!dir.path().join("demo-ls/1.0.0").exists());
    let staging = std::fs::read_dir(dir.path().join(".staging"))
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert_eq!(staging, 0);
    assert!(toolchains.store().list().is_empty());
}

#[test]
fn path_and_home_installs_win_unless_the_project_pins_a_managed_version() {
    let server = FakeServer::start();
    let archive = tool_archive("demo", "demo-ls", b"managed");
    server.serve("/demo.tar.gz", archive.clone());
    let index = ReleaseIndex {
        releases: vec![release(
            "2.0.0",
            server.url("/demo.tar.gz"),
            Checksum::Sha256(sha256_hex(&archive)),
        )],
    };
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("toolchains");
    let mut catalog = Catalog::new();
    catalog.register(Arc::new(StaticProvider {
        spec: spec(),
        index,
    }));
    let http = HttpClient::new();
    let base = toolchains(catalog.clone(), &root);
    let release = base.resolve("demo-ls", None, &http).unwrap();
    base.store()
        .install(&release, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap();

    // A system install on PATH.
    let system_bin = dir.path().join("usr/bin");
    std::fs::create_dir_all(&system_bin).unwrap();
    let system_exe = system_bin.join("demo-ls");
    std::fs::write(&system_exe, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&system_exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let with_path = Toolchains::new(catalog.clone(), ToolchainStore::new(&root))
        .with_platform(platform())
        .with_environment(system_bin.clone().into_os_string(), BTreeMap::new());
    let located = with_path.locate("demo-ls").unwrap();
    assert_eq!(located.source, LocateSource::Path);
    assert_eq!(located.executable, system_exe);

    // A home variable beats PATH.
    let home = dir.path().join("opt/demo");
    std::fs::create_dir_all(home.join("bin")).unwrap();
    std::fs::copy(&system_exe, home.join("bin/demo-ls")).unwrap();
    let with_home = Toolchains::new(catalog.clone(), ToolchainStore::new(&root))
        .with_platform(platform())
        .with_environment(
            system_bin.clone().into_os_string(),
            BTreeMap::from([("DEMO_HOME".into(), home.clone().into_os_string())]),
        );
    let located = with_home.locate("demo-ls").unwrap();
    assert_eq!(located.source, LocateSource::HomeEnv("DEMO_HOME".into()));
    assert_eq!(located.home, home);

    // A project pin satisfied by a managed install beats both.
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let mut pinned = with_home.clone();
    let settings = ToolchainSettings::parse(
        "[components.demo-ls]\nversion = \"2\"\n",
        Path::new("toolchains.toml"),
    )
    .unwrap();
    pinned.apply_project(Some(&project), settings);
    let located = pinned.locate("demo-ls").unwrap();
    assert_eq!(located.source, LocateSource::Managed);
    let env = pinned.environment(&["demo-ls"]);
    let path = env.get("PATH").unwrap();
    let first = std::env::split_paths(path).next().unwrap();
    assert_eq!(first, root.join("demo-ls/2.0.0/bin"));
    assert_eq!(
        env.get("DEMO_HOME").unwrap(),
        &root.join("demo-ls/2.0.0").into_os_string()
    );

    // A configured path beats everything.
    let mut configured = pinned.clone();
    let settings = ToolchainSettings::parse(
        &format!(
            "[components.demo-ls]\nversion = \"2\"\npath = {:?}\n",
            home.display().to_string()
        ),
        Path::new("toolchains.toml"),
    )
    .unwrap();
    configured.apply_project(Some(&project), settings);
    assert_eq!(
        configured.locate("demo-ls").unwrap().source,
        LocateSource::Configured
    );
}

#[test]
fn unreachable_hosts_report_offline_and_missing_files_report_http_status() {
    let server = FakeServer::start();
    let http = HttpClient::new();
    // Bind and drop a listener to get a port nothing listens on.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let error = http
        .get_text(&format!("http://127.0.0.1:{port}/index.json"))
        .unwrap_err();
    assert!(matches!(error, ToolchainError::Offline { .. }), "{error}");
    assert!(error.to_string().contains("HTTPS_PROXY"));
    let error = http.get_text(&server.url("/missing")).unwrap_err();
    assert!(
        matches!(error, ToolchainError::Http { status: 404, .. }),
        "{error}"
    );
}

#[test]
fn service_installs_in_the_background_and_reports_progress() {
    let server = FakeServer::start();
    let archive = tool_archive("demo", "demo-ls", &noise(200_000));
    server.serve("/demo.tar.gz", archive.clone());
    let index = ReleaseIndex {
        releases: vec![release(
            "3.1.0",
            server.url("/demo.tar.gz"),
            Checksum::Sha256(sha256_hex(&archive)),
        )],
    };
    let dir = tempfile::tempdir().unwrap();
    let toolchains = toolchains(index_catalog(&server, &index), dir.path());
    let mut service = ToolchainService::default();
    let job = service.install(&toolchains, "demo-ls", None);
    // A second request for the same component joins the running job.
    assert_eq!(service.install(&toolchains, "demo-ls", None), job);
    assert!(service.is_busy());
    let (events, result) = service.wait(job, Duration::from_secs(30));
    let installed = result.expect("job finished").expect("install succeeded");
    assert_eq!(installed.version.as_str(), "3.1.0");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ToolchainEvent::Resolved { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ToolchainEvent::Progress { .. }))
    );
    assert!(!service.is_busy());

    let unknown = service.install(&toolchains, "nope", None);
    let (_, result) = service.wait(unknown, Duration::from_secs(10));
    assert!(
        result
            .unwrap()
            .unwrap_err()
            .contains("unknown toolchain component")
    );
}
