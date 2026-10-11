//! `.jtplugin` archives: a zip file with `archipel-plugin.toml` (or `jetthing-plugin.toml`) at its root
//! (or inside a single top-level directory) plus the plugin's files.
//!
//! Installation extracts into `<plugins dir>/<plugin id>/` through a temporary
//! directory, so a failed install never leaves a half-written plugin behind.

use crate::{JETTHING_MANIFEST_FILE, MANIFEST_FILE, PluginError, PluginKind, PluginManifest};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

/// File extension of plugin archives.
pub const ARCHIVE_EXTENSION: &str = "jtplugin";
const MAX_ENTRIES: usize = 10_000;
/// Manifest names accepted in archives, in order of preference.
const ARCHIVE_MANIFESTS: [&str; 2] = [MANIFEST_FILE, JETTHING_MANIFEST_FILE];

const MAX_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;

fn io_error(path: &Path, error: impl std::fmt::Display) -> PluginError {
    PluginError::Io(path.display().to_string(), error.to_string())
}

fn zip_error(path: &Path, error: zip::result::ZipError) -> PluginError {
    PluginError::Manifest(format!(
        "{} is not a valid .jtplugin archive: {error}",
        path.display()
    ))
}

/// Archive-relative path of every regular file, with the common top-level
/// directory (if the manifest lives in one) stripped.
fn entry_paths(
    archive: &mut zip::ZipArchive<File>,
    source: &Path,
) -> Result<(Vec<(usize, PathBuf)>, PathBuf), PluginError> {
    if archive.len() > MAX_ENTRIES {
        return Err(PluginError::Manifest(format!(
            "{} has {} entries; the limit is {MAX_ENTRIES}",
            source.display(),
            archive.len()
        )));
    }
    let mut files = Vec::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|e| zip_error(source, e))?;
        let Some(path) = entry.enclosed_name() else {
            return Err(PluginError::Manifest(format!(
                "{}: entry `{}` escapes the plugin directory",
                source.display(),
                entry.name()
            )));
        };
        if entry.is_symlink() {
            return Err(PluginError::Manifest(format!(
                "{}: symbolic links are not allowed (`{}`)",
                source.display(),
                entry.name()
            )));
        }
        total = total.saturating_add(entry.size());
        if total > MAX_UNPACKED_BYTES {
            return Err(PluginError::Manifest(format!(
                "{} unpacks to more than {} MiB",
                source.display(),
                MAX_UNPACKED_BYTES / 1024 / 1024
            )));
        }
        if entry.is_file() {
            files.push((index, path));
        }
    }
    let root = if files
        .iter()
        .any(|(_, path)| ARCHIVE_MANIFESTS.iter().any(|name| path == Path::new(name)))
    {
        PathBuf::new()
    } else {
        let candidates: Vec<&PathBuf> = files
            .iter()
            .map(|(_, path)| path)
            .filter(|path| {
                path.components().count() == 2
                    && ARCHIVE_MANIFESTS.iter().any(|name| path.ends_with(name))
            })
            .collect();
        match candidates.as_slice() {
            [manifest] => manifest.parent().map(Path::to_path_buf).unwrap_or_default(),
            [] => {
                return Err(PluginError::Manifest(format!(
                    "{} contains no {MANIFEST_FILE} (or {JETTHING_MANIFEST_FILE}) at its root",
                    source.display()
                )));
            }
            _ => {
                return Err(PluginError::Manifest(format!(
                    "{} contains several plugins; package one plugin per archive",
                    source.display()
                )));
            }
        }
    };
    let files = files
        .into_iter()
        .filter_map(|(index, path)| {
            path.strip_prefix(&root)
                .ok()
                .map(|relative| (index, relative.to_path_buf()))
        })
        .collect();
    Ok((files, root))
}

/// Reads and validates the manifest of an archive without extracting it.
pub fn read_manifest(source: &Path) -> Result<PluginManifest, PluginError> {
    let file = File::open(source).map_err(|e| io_error(source, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| zip_error(source, e))?;
    let (_, root) = entry_paths(&mut archive, source)?;
    let mut text = String::new();
    let name = ARCHIVE_MANIFESTS
        .iter()
        .map(|name| root.join(name).to_string_lossy().replace('\\', "/"))
        .find(|name| archive.by_name(name).is_ok())
        .unwrap_or_else(|| MANIFEST_FILE.to_string());
    archive
        .by_name(&name)
        .map_err(|e| zip_error(source, e))?
        .take(1024 * 1024)
        .read_to_string(&mut text)
        .map_err(|e| io_error(source, e))?;
    let manifest = PluginManifest::parse(&text)?;
    if manifest.kind == PluginKind::Rust {
        return Err(PluginError::Manifest(format!(
            "{} is a Rust plugin; Rust plugins are compiled into Jetthing. Third-party native code must be packaged as a WASM component (kind = \"wasm\")",
            manifest.id
        )));
    }
    Ok(manifest)
}

/// Installs (or replaces) the plugin in `source` under `plugins_dir/<id>`.
pub fn install(source: &Path, plugins_dir: &Path) -> Result<PluginManifest, PluginError> {
    let manifest = read_manifest(source)?;
    let file = File::open(source).map_err(|e| io_error(source, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| zip_error(source, e))?;
    let (files, _) = entry_paths(&mut archive, source)?;
    if let Some(entry) = &manifest.entry
        && !files.iter().any(|(_, path)| path == Path::new(entry))
    {
        return Err(PluginError::Manifest(format!(
            "{}: entry `{entry}` is missing from the archive",
            manifest.id
        )));
    }
    fs::create_dir_all(plugins_dir).map_err(|e| io_error(plugins_dir, e))?;
    let target = plugins_dir.join(&manifest.id);
    let staging = plugins_dir.join(format!(".{}.installing", manifest.id));
    let _ = fs::remove_dir_all(&staging);
    let result = (|| {
        for (index, relative) in &files {
            let destination = staging.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
            }
            let mut entry = archive.by_index(*index).map_err(|e| zip_error(source, e))?;
            let mut output = File::create(&destination).map_err(|e| io_error(&destination, e))?;
            io::copy(&mut (&mut entry).take(MAX_UNPACKED_BYTES), &mut output)
                .map_err(|e| io_error(&destination, e))?;
        }
        replace_dir(&staging, &target)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result.map(|()| manifest)
}

/// Moves `staging` to `target`, replacing an existing installation.
fn replace_dir(staging: &Path, target: &Path) -> Result<(), PluginError> {
    let backup = target.with_extension("previous");
    let _ = fs::remove_dir_all(&backup);
    let had_previous = target.exists();
    if had_previous {
        fs::rename(target, &backup).map_err(|e| io_error(target, e))?;
    }
    if let Err(error) = fs::rename(staging, target) {
        if had_previous {
            let _ = fs::rename(&backup, target);
        }
        return Err(io_error(target, error));
    }
    let _ = fs::remove_dir_all(&backup);
    Ok(())
}

/// Removes an installed plugin directory. Only direct children of
/// `plugins_dir` named after a valid plugin id are removed.
pub fn uninstall(plugins_dir: &Path, id: &str) -> Result<(), PluginError> {
    if !crate::valid_id(id) {
        return Err(PluginError::InvalidId(id.into()));
    }
    let target = plugins_dir.join(id);
    if !ARCHIVE_MANIFESTS
        .iter()
        .any(|name| target.join(name).is_file())
    {
        return Err(PluginError::NotFound(format!(
            "{id} is not installed in {}",
            plugins_dir.display()
        )));
    }
    fs::remove_dir_all(&target).map_err(|e| io_error(&target, e))
}

/// Packs the plugin directory `dir` into `output` (a `.jtplugin` file).
/// Hidden files and `target/` are skipped.
pub fn pack(dir: &Path, output: &Path) -> Result<PluginManifest, PluginError> {
    let manifest_path = ARCHIVE_MANIFESTS
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| dir.join(MANIFEST_FILE));
    let text = fs::read_to_string(&manifest_path).map_err(|e| io_error(&manifest_path, e))?;
    let manifest = PluginManifest::parse(&text)?;
    let mut files = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    }
    let file = File::create(output).map_err(|e| io_error(output, e))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for relative in files {
        let name = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        writer
            .start_file(name, options)
            .map_err(|e| zip_error(output, e))?;
        let bytes = fs::read(dir.join(&relative)).map_err(|e| io_error(&relative, e))?;
        writer.write_all(&bytes).map_err(|e| io_error(output, e))?;
    }
    writer.finish().map_err(|e| zip_error(output, e))?;
    Ok(manifest)
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), PluginError> {
    for entry in fs::read_dir(dir).map_err(|e| io_error(dir, e))? {
        let entry = entry.map_err(|e| io_error(dir, e))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let kind = entry.file_type().map_err(|e| io_error(&path, e))?;
        if kind.is_dir() {
            collect(root, &path, files)?;
        } else if kind.is_file()
            && let Ok(relative) = path.strip_prefix(root)
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        {
            files.push(relative.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("jetthing-archive-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_zip(path: &Path, entries: &[(&str, &str)]) {
        let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, body) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(body.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }

    const LUA: &str =
        "id='x.tool'\nname='Tool'\nversion='1.0'\napi_version=1\nkind='lua'\nentry='main.lua'\n";

    #[test]
    fn packs_installs_replaces_and_uninstalls() {
        let base = temp("roundtrip");
        let source = base.join("src");
        fs::create_dir_all(source.join("lib")).unwrap();
        fs::write(source.join(MANIFEST_FILE), LUA).unwrap();
        fs::write(source.join("main.lua"), "-- v1").unwrap();
        fs::write(source.join("lib/util.lua"), "return {}").unwrap();
        fs::create_dir_all(source.join(".git")).unwrap();
        fs::write(source.join(".git/HEAD"), "x").unwrap();
        let archive = base.join("tool.jtplugin");
        assert_eq!(pack(&source, &archive).unwrap().id, "x.tool");
        let plugins = base.join("plugins");
        install(&archive, &plugins).unwrap();
        assert_eq!(
            fs::read_to_string(plugins.join("x.tool/main.lua")).unwrap(),
            "-- v1"
        );
        assert!(plugins.join("x.tool/lib/util.lua").is_file());
        assert!(!plugins.join("x.tool/.git").exists());
        // Reinstalling a newer archive replaces the files.
        fs::write(source.join("main.lua"), "-- v2").unwrap();
        pack(&source, &archive).unwrap();
        install(&archive, &plugins).unwrap();
        assert_eq!(
            fs::read_to_string(plugins.join("x.tool/main.lua")).unwrap(),
            "-- v2"
        );
        assert_eq!(
            fs::read_dir(&plugins).unwrap().count(),
            1,
            "no staging leftovers"
        );
        uninstall(&plugins, "x.tool").unwrap();
        assert!(!plugins.join("x.tool").exists());
        assert!(uninstall(&plugins, "x.tool").is_err());
        assert!(uninstall(&plugins, "../etc").is_err());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn accepts_a_single_top_level_directory() {
        let base = temp("nested");
        let archive = base.join("nested.jtplugin");
        write_zip(
            &archive,
            &[("tool/jetthing-plugin.toml", LUA), ("tool/main.lua", "")],
        );
        install(&archive, &base.join("plugins")).unwrap();
        assert!(base.join("plugins/x.tool/main.lua").is_file());
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn rejects_unsafe_or_invalid_archives() {
        let base = temp("invalid");
        let plugins = base.join("plugins");
        type Case<'a> = (&'a str, Vec<(&'a str, &'a str)>, &'a str);
        let cases: [Case; 6] = [
            (
                "traversal",
                vec![(MANIFEST_FILE, LUA), ("../evil.lua", "")],
                "escapes",
            ),
            (
                "missing",
                vec![("readme.md", "")],
                "no archipel-plugin.toml",
            ),
            (
                "entry",
                vec![(MANIFEST_FILE, LUA)],
                "missing from the archive",
            ),
            (
                "rust",
                vec![(
                    MANIFEST_FILE,
                    "id='x.r'\nname='R'\nversion='1'\napi_version=1\nkind='rust'\n",
                )],
                "WASM component",
            ),
            (
                "two",
                vec![
                    ("a/jetthing-plugin.toml", LUA),
                    ("b/jetthing-plugin.toml", LUA),
                ],
                "several plugins",
            ),
            (
                "badid",
                vec![(
                    MANIFEST_FILE,
                    "id='nodot'\nname='N'\nversion='1'\napi_version=1\n",
                )],
                "reverse-domain",
            ),
        ];
        for (name, entries, needle) in cases {
            let archive = base.join(format!("{name}.jtplugin"));
            write_zip(&archive, &entries);
            let error = install(&archive, &plugins).unwrap_err().to_string();
            assert!(error.contains(needle), "{name}: {error}");
        }
        fs::write(base.join("junk.jtplugin"), "not a zip").unwrap();
        assert!(
            install(&base.join("junk.jtplugin"), &plugins)
                .unwrap_err()
                .to_string()
                .contains("not a valid")
        );
        assert!(!base.join("evil.lua").exists());
        assert!(!plugins.exists() || fs::read_dir(&plugins).unwrap().count() == 0);
        fs::remove_dir_all(&base).unwrap();
    }
}
