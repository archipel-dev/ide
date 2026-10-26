//! Built-in providers against recorded-shape responses on a local server.

mod support;

use archipel_ide_toolchains::download::{HttpClient, sha256_hex};
use archipel_ide_toolchains::providers::github::GithubReleaseProvider;
use archipel_ide_toolchains::providers::{
    gradle::GradleDistributions, jdk::FoojayJdk, jdtls::EclipseJdtls, kotlin_lsp::KotlinLsp,
    rust_analyzer,
};
use archipel_ide_toolchains::{
    ArchiveKind, Checksum, ComponentProvider, ComponentSettings, Platform, ToolchainError,
    VersionRequest,
};
use std::sync::atomic::AtomicBool;
use support::FakeServer;

fn linux() -> Platform {
    Platform::parse("linux", "x86_64").unwrap()
}

fn foojay(server: &FakeServer) {
    server.serve(
        "/disco/major_versions",
        r#"{"result":[{"major_version":25,"term_of_support":"LTS"},{"major_version":27,"term_of_support":"STS"},{"major_version":21,"term_of_support":"LTS"}]}"#,
    );
    let package = |id: &str, version: &str, libc: &str, fx: bool| {
        format!(
            r#"{{"java_version":"{version}","archive_type":"tar.gz","javafx_bundled":{fx},"lib_c_type":"{libc}","size":100,"links":{{"pkg_info_uri":"{}"}}}}"#,
            server.url(&format!("/disco/ids/{id}"))
        )
    };
    server.serve(
        "/disco/packages",
        format!(
            r#"{{"result":[{},{},{},{}]}}"#,
            package("a", "21.0.3+9", "glibc", false),
            package("b", "21.0.4+7", "glibc", false),
            package("c", "21.0.5+1", "musl", false),
            package("d", "21.0.9+1", "glibc", true),
        ),
    );
    for (id, file) in [("a", "jdk-21.0.3.tar.gz"), ("b", "jdk-21.0.4.tar.gz")] {
        server.serve(
            &format!("/disco/ids/{id}"),
            format!(
                r#"{{"result":[{{"filename":"{file}","direct_download_uri":"{}","checksum":"{}","checksum_type":"sha256","checksum_uri":""}}]}}"#,
                server.url(&format!("/dl/{file}")),
                "ab".repeat(32)
            ),
        );
    }
}

#[test]
fn foojay_picks_newest_matching_glibc_jdk_without_javafx() {
    let server = FakeServer::start();
    foojay(&server);
    let provider = FoojayJdk::with_api(&server.url("/disco"));
    let http = HttpClient::new();
    let release = provider
        .resolve(&VersionRequest::parse("21"), &linux(), &http, None)
        .unwrap();
    assert_eq!(release.version.as_str(), "21.0.4+7");
    assert_eq!(release.artifact.url, server.url("/dl/jdk-21.0.4.tar.gz"));
    assert_eq!(release.artifact.checksum, Checksum::Sha256("ab".repeat(32)));
    assert_eq!(release.artifact.archive, ArchiveKind::TarGz);
    let query = &server.requests("/disco/packages")[0].path;
    assert!(query.contains("version=21&distribution=temurin"), "{query}");
    assert!(query.contains("lib_c_type=glibc") && query.contains("latest=available"));

    // `lts` resolves the newest LTS feature release first.
    let _ = provider.resolve(&VersionRequest::parse("lts"), &linux(), &http, None);
    let query = &server.requests("/disco/packages")[1].path;
    assert!(query.contains("version=25&"), "{query}");

    // Distribution from settings; invalid names are refused before any request.
    let settings = ComponentSettings {
        distribution: Some("zulu".into()),
        ..Default::default()
    };
    let _ = provider.resolve(
        &VersionRequest::parse("21.0.3"),
        &linux(),
        &http,
        Some(&settings),
    );
    let query = &server.requests("/disco/packages")[2].path;
    assert!(query.contains("version=21.0.3&distribution=zulu") && !query.contains("latest="));
    let bad = ComponentSettings {
        distribution: Some("zulu&x=1".into()),
        ..Default::default()
    };
    assert!(matches!(
        provider.resolve(&VersionRequest::Latest, &linux(), &http, Some(&bad)),
        Err(ToolchainError::Settings { .. })
    ));
    assert!(matches!(
        provider.resolve(&VersionRequest::parse("11"), &linux(), &http, None),
        Err(ToolchainError::NoMatchingVersion { .. })
    ));
}

#[test]
fn jdtls_resolves_milestones_and_installs_with_sidecar_checksum() {
    let server = FakeServer::start();
    // jdtls tarballs have no single root directory.
    let archive = {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for (path, data) in [
            ("bin/jdtls", &b"launcher"[..]),
            ("plugins/x.jar", &b"jar"[..]),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, path, data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    };
    server.serve(
        "/jdtls/milestones/",
        "<a href='/jdtls/milestones/1.9.0'> 1.9.0</a><a href='/jdtls/milestones/1.40.0'>1.40.0</a>",
    );
    server.serve(
        "/jdtls/milestones/1.40.0/latest.txt",
        "jdt-language-server-1.40.0-202409261450.tar.gz\n",
    );
    let file = "/jdtls/milestones/1.40.0/jdt-language-server-1.40.0-202409261450.tar.gz";
    server.serve(file, archive.clone());
    server.serve(&format!("{file}.sha256"), sha256_hex(&archive));
    let provider = EclipseJdtls::with_base(&server.url("/jdtls"));
    let http = HttpClient::new();
    let release = provider
        .resolve(&VersionRequest::Latest, &linux(), &http, None)
        .unwrap();
    assert_eq!(release.version.as_str(), "1.40.0");
    assert_eq!(
        release.artifact.checksum,
        Checksum::Sha256Url(server.url(&format!("{file}.sha256")))
    );
    let dir = tempfile::tempdir().unwrap();
    let store = archipel_ide_toolchains::ToolchainStore::new(dir.path());
    let installed = store
        .install(&release, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap();
    assert!(installed.path.join("bin/jdtls").is_file());
    assert!(installed.path.join("plugins/x.jar").is_file());

    server.serve("/jdtls/milestones/1.9.0/latest.txt", "../../evil.tar.gz");
    assert!(matches!(
        provider.resolve(&VersionRequest::parse("1.9"), &linux(), &http, None),
        Err(ToolchainError::Catalog { .. })
    ));
}

#[test]
fn kotlin_lsp_skips_versions_without_archives_and_falls_back_to_fwcd() {
    let server = FakeServer::start();
    server.serve(
        "/RELEASES.md",
        "### v263.9999.0\n### v263.6379.0\n### v262.9593.0\n",
    );
    let hex = "ab".repeat(32);
    let file = "/kotlin-server/263.6379.0/kotlin-server-263.6379.0.tar.gz";
    server.serve(
        &format!("{file}.sha256"),
        format!("{hex} *kotlin-server-263.6379.0.tar.gz"),
    );
    let provider = KotlinLsp::with_sources(
        &server.url("/kotlin-server"),
        &server.url("/RELEASES.md"),
        &server.url("/api"),
    );
    let http = HttpClient::new();
    // 263.9999.0 has no sidecar (404) so the next version is chosen.
    let release = provider
        .resolve(&VersionRequest::Latest, &linux(), &http, None)
        .unwrap();
    assert_eq!(release.version.as_str(), "263.6379.0");
    assert_eq!(release.artifact.url, server.url(file));
    assert_eq!(
        release.artifact.checksum,
        Checksum::Sha256Url(server.url(&format!("{file}.sha256")))
    );
    // A version that is not listed is an error, not a silent switch of server.
    assert!(matches!(
        provider.resolve(&VersionRequest::parse("1.0"), &linux(), &http, None),
        Err(ToolchainError::NoMatchingVersion { .. })
    ));
    // macOS has no extractable JetBrains archive: fwcd's server.zip (with its digest) is used.
    server.serve(
        "/api/repos/fwcd/kotlin-language-server/releases/latest",
        github_release(&server, "1.3.13", &[("server.zip", Some(hex.clone()))]),
    );
    let mac = Platform::parse("macos", "aarch64").unwrap();
    let release = provider
        .resolve(&VersionRequest::Latest, &mac, &http, None)
        .unwrap();
    assert_eq!(release.version.as_str(), "1.3.13");
    assert_eq!(release.artifact.checksum, Checksum::Sha256(hex));
}

#[test]
fn gradle_resolves_current_and_prefix_versions_with_published_checksums() {
    let server = FakeServer::start();
    // The binary distribution has a single `gradle-<v>/` root.
    let archive = {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file("gradle-9.1.0/bin/gradle", options).unwrap();
        std::io::Write::write_all(&mut zip, b"#!/bin/sh\n").unwrap();
        zip.start_file("gradle-9.1.0/lib/gradle-launcher.jar", options)
            .unwrap();
        std::io::Write::write_all(&mut zip, b"jar").unwrap();
        zip.finish().unwrap().into_inner()
    };
    let zip_path = "/distributions/gradle-9.1.0-bin.zip";
    server.serve(zip_path, archive.clone());
    server.serve(&format!("{zip_path}.sha256"), sha256_hex(&archive));
    let entry = |version: &str, extra: &str| {
        format!(
            r#"{{"version":"{version}","snapshot":false,"nightly":false,"releaseNightly":false,"activeRc":false,"rcFor":"","milestoneFor":"","broken":false,"downloadUrl":"{}","checksumUrl":"{}"{extra}}}"#,
            server.url(&format!("/distributions/gradle-{version}-bin.zip")),
            server.url(&format!("/distributions/gradle-{version}-bin.zip.sha256")),
        )
    };
    server.serve("/gradle/versions/current", entry("9.1.0", ""));
    server.serve(
        "/gradle/versions/all",
        format!(
            "[{},{},{},{}]",
            entry("9.2.0-rc-1", "").replace(r#""rcFor":"""#, r#""rcFor":"9.2.0""#),
            entry("9.1.0", ""),
            entry("8.14.3", ""),
            entry("8.14.2", ""),
        ),
    );
    let provider = GradleDistributions::with_base(&server.url("/gradle"));
    let http = HttpClient::new();
    let release = provider
        .resolve(&VersionRequest::Latest, &linux(), &http, None)
        .unwrap();
    assert_eq!(release.version.as_str(), "9.1.0");
    assert_eq!(release.artifact.archive, ArchiveKind::Zip);
    assert_eq!(
        release.artifact.checksum,
        Checksum::Sha256Url(server.url(&format!("{zip_path}.sha256")))
    );
    // Prefix requests use the full listing and skip release candidates.
    let older = provider
        .resolve(&VersionRequest::parse("8.14"), &linux(), &http, None)
        .unwrap();
    assert_eq!(older.version.as_str(), "8.14.3");
    assert!(matches!(
        provider.resolve(&VersionRequest::parse("9.2"), &linux(), &http, None),
        Err(ToolchainError::NoMatchingVersion { .. })
    ));
    // Installing verifies the published SHA-256 and strips the root directory.
    let dir = tempfile::tempdir().unwrap();
    let store = archipel_ide_toolchains::ToolchainStore::new(dir.path());
    let installed = store
        .install(&release, &http, &mut |_| {}, &AtomicBool::new(false))
        .unwrap();
    let launcher = installed.path.join("bin/gradle");
    assert!(launcher.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&launcher).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "bin/gradle stays executable");
    }
    // A download URL that is not the named distribution is refused.
    server.serve(
        "/gradle/versions/current",
        r#"{"version":"9.1.0","downloadUrl":"https://example.com/evil.zip","checksumUrl":"https://example.com/evil.zip.sha256"}"#,
    );
    assert!(matches!(
        provider.resolve(&VersionRequest::Latest, &linux(), &http, None),
        Err(ToolchainError::Catalog { .. })
    ));
}

fn github_release(server: &FakeServer, tag: &str, assets: &[(&str, Option<String>)]) -> String {
    let assets: Vec<String> = assets
        .iter()
        .map(|(name, digest)| {
            format!(
                r#"{{"name":"{name}","browser_download_url":"{}","size":3,"digest":{}}}"#,
                server.url(&format!("/dl/{tag}/{name}")),
                digest
                    .as_ref()
                    .map_or("null".to_string(), |d| format!("\"sha256:{d}\""))
            )
        })
        .collect();
    format!(
        r#"{{"tag_name":"{tag}","draft":false,"assets":[{}]}}"#,
        assets.join(",")
    )
}

#[test]
fn github_releases_use_asset_digests_or_checksum_assets() {
    let server = FakeServer::start();
    let digest = "cd".repeat(32);
    server.serve(
        "/api/repos/rust-lang/rust-analyzer/releases/latest",
        github_release(
            &server,
            "2026-10-06",
            &[
                (
                    "rust-analyzer-x86_64-unknown-linux-gnu.gz",
                    Some(digest.clone()),
                ),
                ("rust-analyzer-x86_64-apple-darwin.gz", None),
            ],
        ),
    );
    server.serve(
        "/api/repos/rust-lang/rust-analyzer/releases/tags/nightly",
        github_release(
            &server,
            "nightly",
            &[("rust-analyzer-x86_64-apple-darwin.gz", None)],
        ),
    );
    let provider = rust_analyzer().with_api(&server.url("/api"));
    let http = HttpClient::new();
    let release = provider
        .resolve(&VersionRequest::Latest, &linux(), &http, None)
        .unwrap();
    assert_eq!(release.version.as_str(), "2026-10-06");
    assert_eq!(release.artifact.checksum, Checksum::Sha256(digest));
    assert!(matches!(release.artifact.archive, ArchiveKind::Gz { .. }));

    // No digest and no checksum asset: refuse instead of installing unverified.
    let mac = Platform::parse("macos", "aarch64").unwrap();
    assert!(matches!(
        provider.resolve(&VersionRequest::Latest, &mac, &http, None),
        Err(ToolchainError::UnsupportedPlatform { .. })
    ));
    let intel_mac = Platform::parse("macos", "x86_64").unwrap();
    let error = provider
        .resolve(&VersionRequest::parse("nightly"), &intel_mac, &http, None)
        .unwrap_err();
    assert!(error.to_string().contains("refusing"), "{error}");

    // `v`-prefixed tags and checksum-list assets.
    let tool = GithubReleaseProvider::new(
        archipel_ide_toolchains::ComponentSpec::new(
            "demo",
            "Demo",
            archipel_ide_toolchains::ComponentKind::Formatter,
            "demo",
        ),
        "o/demo",
        |_, tag| Some((format!("demo-{tag}.tar.gz"), ArchiveKind::TarGz)),
    )
    .with_api(&server.url("/api"));
    server.serve(
        "/api/repos/o/demo/releases/tags/v1.2.3",
        github_release(
            &server,
            "v1.2.3",
            &[("demo-v1.2.3.tar.gz", None), ("checksums.txt", None)],
        ),
    );
    let release = tool
        .resolve(&VersionRequest::parse("1.2.3"), &linux(), &http, None)
        .unwrap();
    assert_eq!(release.version.as_str(), "1.2.3");
    assert_eq!(
        release.artifact.checksum,
        Checksum::Sha256Url(server.url("/dl/v1.2.3/checksums.txt"))
    );
    assert!(matches!(
        tool.resolve(&VersionRequest::parse("9.9.9"), &linux(), &http, None),
        Err(ToolchainError::NoMatchingVersion { .. })
    ));
}

/// Live network check of the real JDK + jdtls sources; run with `--ignored`.
#[test]
#[ignore = "downloads ~380 MB from api.foojay.io and download.eclipse.org"]
fn live_jdk_and_jdtls_install() {
    let dir = tempfile::tempdir().unwrap();
    let store = archipel_ide_toolchains::ToolchainStore::new(dir.path());
    let http = HttpClient::new();
    let platform = Platform::current().expect("supported platform");
    for provider in [
        Box::new(FoojayJdk::default()) as Box<dyn ComponentProvider>,
        Box::new(EclipseJdtls::default()),
    ] {
        let release = provider
            .resolve(&VersionRequest::parse("21"), &platform, &http, None)
            .or_else(|_| provider.resolve(&VersionRequest::Latest, &platform, &http, None))
            .unwrap();
        let installed = store
            .install(&release, &http, &mut |_| {}, &AtomicBool::new(false))
            .unwrap();
        assert!(
            installed.executable(provider.spec(), &platform).is_some(),
            "{} has no executable",
            provider.spec().id
        );
    }
}
