//! Java versions that OSGi bundles (jdtls, Eclipse plugins) declare they need.
//!
//! A bundle names its minimum Java in its manifest, either as
//! `Bundle-RequiredExecutionEnvironment: JavaSE-25` (any listed one suffices) or as
//! `Require-Capability: osgi.ee;filter:="(&(osgi.ee=JavaSE)(version=25))"`. An
//! application such as jdtls needs the highest of its bundles' minimums: releases
//! may raise it in platform bundles while the main bundle still says an older one.

use std::io::Read;
use std::path::Path;

/// Highest minimum Java major version among the `*.jar` bundles in `plugins`.
pub fn required_java_major(plugins: &Path) -> Option<u32> {
    std::fs::read_dir(plugins)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jar"))
        .filter_map(|jar| bundle_manifest(&jar))
        .filter_map(|manifest| manifest_java_major(&manifest))
        .max()
}

fn bundle_manifest(jar: &Path) -> Option<String> {
    let file = std::fs::File::open(jar).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name("META-INF/MANIFEST.MF").ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

/// The minimum Java major version one manifest declares.
pub fn manifest_java_major(manifest: &str) -> Option<u32> {
    let headers = unfold(manifest);
    let from_bree = headers
        .iter()
        .find_map(|(name, value)| {
            name.eq_ignore_ascii_case("Bundle-RequiredExecutionEnvironment")
                .then_some(value)
        })
        .and_then(|value| {
            // Any listed environment suffices, so the bundle needs the lowest.
            value
                .split(',')
                .filter_map(|env| env.trim().strip_prefix("JavaSE-"))
                .filter_map(java_major)
                .min()
        });
    let from_capability = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("Require-Capability"))
        .find_map(|(_, value)| {
            let ee = value.find("osgi.ee=JavaSE")?;
            let rest = &value[ee..];
            let version = rest.find("version=")? + "version=".len();
            let digits: String = rest[version..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            java_major(&digits)
        });
    from_bree.max(from_capability)
}

/// `25`, `21`, `1.8` → 8.
fn java_major(text: &str) -> Option<u32> {
    let mut parts = text.split('.');
    let first: u32 = parts.next()?.trim().parse().ok()?;
    if first == 1 {
        return parts.next()?.trim().parse().ok();
    }
    Some(first)
}

/// Manifest headers with continuation lines (leading space) joined.
fn unfold(manifest: &str) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = Vec::new();
    for line in manifest.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix(' ') {
            if let Some((_, value)) = headers.last_mut() {
                value.push_str(rest);
            }
        } else if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_execution_environment_header() {
        let manifest =
            "Manifest-Version: 1.0\r\nBundle-RequiredExecutionEnvironment: JavaSE-25\r\n";
        assert_eq!(manifest_java_major(manifest), Some(25));
        assert_eq!(
            manifest_java_major("Bundle-RequiredExecutionEnvironment: JavaSE-1.8\n"),
            Some(8)
        );
        // Any listed environment suffices.
        assert_eq!(
            manifest_java_major("Bundle-RequiredExecutionEnvironment: JavaSE-17, JavaSE-21\n"),
            Some(17)
        );
    }

    #[test]
    fn reads_folded_capability_filters() {
        let manifest = "Require-Capability: osgi.ee; filter:=\"(&(osgi.ee=JavaSE)(versi\n on=25))\"\nImport-Package: x\n";
        assert_eq!(manifest_java_major(manifest), Some(25));
        assert_eq!(manifest_java_major("Bundle-Name: x\n"), None);
    }

    #[test]
    fn plugins_need_the_highest_bundle_minimum() {
        let dir = std::env::temp_dir().join(format!("archipel-osgi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let jar = |name: &str, manifest: &str| {
            let file = std::fs::File::create(dir.join(name)).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file(
                "META-INF/MANIFEST.MF",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            std::io::Write::write_all(&mut zip, manifest.as_bytes()).unwrap();
            zip.finish().unwrap();
        };
        jar(
            "core.jar",
            "Bundle-RequiredExecutionEnvironment: JavaSE-21\n",
        );
        jar(
            "resources.jar",
            "Bundle-RequiredExecutionEnvironment: JavaSE-25\n",
        );
        jar("lib.jar", "Bundle-Name: no requirement\n");
        std::fs::write(dir.join("notes.txt"), "not a jar").unwrap();
        assert_eq!(required_java_major(&dir), Some(25));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
