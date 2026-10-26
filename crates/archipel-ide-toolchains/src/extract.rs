//! Safe archive extraction. Entries that would escape the destination
//! (absolute paths, `..`, symlinks pointing outside) are rejected or skipped.

use crate::{ArchiveKind, ToolchainError};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Component, Path, PathBuf};

/// Extracts `archive` into `destination` (created if needed). When the archive
/// holds a single top-level directory (`jdk-21.0.4+7/`), its contents are moved
/// up so installs have a predictable layout.
pub fn extract(
    archive: &Path,
    kind: &ArchiveKind,
    destination: &Path,
) -> Result<(), ToolchainError> {
    fs::create_dir_all(destination)?;
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match kind {
        ArchiveKind::TarGz => {
            let decoder = flate2::read::GzDecoder::new(BufReader::new(File::open(archive)?));
            unpack_tar(decoder, destination, &name)?;
        }
        ArchiveKind::TarXz => {
            // lzma-rs decompresses into a writer; stage the tar next to the archive.
            let mut tar_path = archive.as_os_str().to_owned();
            tar_path.push(".untar");
            let tar_path = PathBuf::from(tar_path);
            {
                let mut input = BufReader::new(File::open(archive)?);
                let mut output = BufWriter::new(File::create(&tar_path)?);
                lzma_rs::xz_decompress(&mut input, &mut output)
                    .map_err(|error| ToolchainError::archive(&name, format!("{error:?}")))?;
            }
            let result = unpack_tar(File::open(&tar_path)?, destination, &name);
            let _ = fs::remove_file(&tar_path);
            result?;
        }
        ArchiveKind::Zip => unpack_zip(archive, destination, &name)?,
        ArchiveKind::Gz { file_name } => {
            let target = destination.join(single_file_name(file_name, &name)?);
            let mut decoder = flate2::read::GzDecoder::new(BufReader::new(File::open(archive)?));
            let mut output = File::create(&target)?;
            std::io::copy(&mut decoder, &mut output)
                .map_err(|error| ToolchainError::archive(&name, error))?;
            make_executable(&target)?;
        }
        ArchiveKind::Raw { file_name } => {
            let target = destination.join(single_file_name(file_name, &name)?);
            fs::copy(archive, &target)?;
            make_executable(&target)?;
        }
    }
    if matches!(
        kind,
        ArchiveKind::TarGz | ArchiveKind::TarXz | ArchiveKind::Zip
    ) {
        strip_single_root(destination)?;
    }
    Ok(())
}

fn single_file_name<'a>(file_name: &'a str, archive: &str) -> Result<&'a str, ToolchainError> {
    let path = Path::new(file_name);
    match (path.components().next(), path.components().count()) {
        (Some(Component::Normal(_)), 1) => Ok(file_name),
        _ => Err(ToolchainError::archive(
            archive,
            format!("invalid target file name `{file_name}`"),
        )),
    }
}

fn unpack_tar(
    reader: impl std::io::Read,
    destination: &Path,
    name: &str,
) -> Result<(), ToolchainError> {
    let mut archive = tar::Archive::new(reader);
    archive.set_preserve_permissions(true);
    archive.set_overwrite(true);
    let entries = archive
        .entries()
        .map_err(|error| ToolchainError::archive(name, error))?;
    for entry in entries {
        let mut entry = entry.map_err(|error| ToolchainError::archive(name, error))?;
        let path = entry
            .path()
            .map_err(|error| ToolchainError::archive(name, error))?
            .into_owned();
        if !is_safe_relative(&path) {
            return Err(ToolchainError::archive(
                name,
                format!("entry `{}` escapes the install directory", path.display()),
            ));
        }
        // `unpack_in` additionally refuses to write through symlinks that leave
        // `destination`.
        entry
            .unpack_in(destination)
            .map_err(|error| ToolchainError::archive(name, error))?;
    }
    Ok(())
}

fn unpack_zip(archive: &Path, destination: &Path, name: &str) -> Result<(), ToolchainError> {
    let mut zip = zip::ZipArchive::new(BufReader::new(File::open(archive)?))
        .map_err(|error| ToolchainError::archive(name, error))?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| ToolchainError::archive(name, error))?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(ToolchainError::archive(
                name,
                format!("entry `{}` escapes the install directory", entry.name()),
            ));
        };
        let target = destination.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        if entry.is_symlink() {
            let mut link = String::new();
            std::io::Read::read_to_string(&mut entry, &mut link)
                .map_err(|error| ToolchainError::archive(name, error))?;
            let resolved = relative.parent().unwrap_or(Path::new("")).join(&link);
            if Path::new(&link).is_absolute() || !is_safe_relative(&normalize(&resolved)) {
                continue;
            }
            create_symlink(Path::new(&link), &target)?;
            continue;
        }
        let mut output = File::create(&target)?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| ToolchainError::archive(name, error))?;
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(mode & 0o7777))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(link: &Path, target: &Path) -> Result<(), ToolchainError> {
    let _ = fs::remove_file(target);
    std::os::unix::fs::symlink(link, target)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_symlink(_link: &Path, _target: &Path) -> Result<(), ToolchainError> {
    // Windows archives rarely contain symlinks and creating them needs
    // privileges; skip.
    Ok(())
}

/// Lexically resolves `.` and `..` components.
fn normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir if matches!(parts.last(), Some(Component::Normal(_))) => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

fn is_safe_relative(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn strip_single_root(destination: &Path) -> Result<(), ToolchainError> {
    let entries: Vec<_> = fs::read_dir(destination)?.collect::<Result<_, _>>()?;
    let [only] = entries.as_slice() else {
        return Ok(());
    };
    if !only.file_type()?.is_dir() {
        return Ok(());
    }
    let inner = only.path();
    let mut lifted = destination.as_os_str().to_owned();
    lifted.push(".lift");
    let lifted = PathBuf::from(lifted);
    let _ = fs::remove_dir_all(&lifted);
    fs::rename(&inner, &lifted)?;
    fs::remove_dir(destination)?;
    fs::rename(&lifted, destination)?;
    Ok(())
}

pub(crate) fn make_executable(path: &Path) -> Result<(), ToolchainError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tar_gz(entries: &[(&str, &[u8], u32)]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for (path, data, mode) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(*mode);
            header.set_cksum();
            builder.append_data(&mut header, path, *data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn tar_gz_strips_single_root_and_keeps_modes() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.tar.gz");
        fs::write(
            &archive,
            tar_gz(&[
                ("jdk-21/bin/java", b"#!/bin/sh\n", 0o755),
                ("jdk-21/release", b"JAVA_VERSION=21", 0o644),
            ]),
        )
        .unwrap();
        let out = dir.path().join("out");
        extract(&archive, &ArchiveKind::TarGz, &out).unwrap();
        assert!(out.join("bin/java").is_file());
        assert!(out.join("release").is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(out.join("bin/java"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn zip_rejects_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("evil.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
        writer
            .start_file("../evil.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, b"x").unwrap();
        writer.finish().unwrap();
        let error = extract(&archive, &ArchiveKind::Zip, &dir.path().join("out")).unwrap_err();
        assert!(matches!(error, ToolchainError::Archive { .. }), "{error}");
        assert!(!dir.path().join("evil.txt").exists());
    }

    #[test]
    fn zip_and_xz_and_single_files_extract() {
        let dir = tempfile::tempdir().unwrap();
        // zip with two roots: not stripped.
        let zip_path = dir.path().join("a.zip");
        let mut writer = zip::ZipWriter::new(File::create(&zip_path).unwrap());
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        writer.start_file("bin/tool", options).unwrap();
        std::io::Write::write_all(&mut writer, b"tool").unwrap();
        writer.start_file("README", options).unwrap();
        writer.finish().unwrap();
        let out = dir.path().join("zip");
        extract(&zip_path, &ArchiveKind::Zip, &out).unwrap();
        assert_eq!(fs::read(out.join("bin/tool")).unwrap(), b"tool");

        // tar.xz
        let tar = {
            let mut builder = tar::Builder::new(Vec::new());
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "node-v20/bin/node", &b"node"[..])
                .unwrap();
            builder.into_inner().unwrap()
        };
        let mut xz = Vec::new();
        lzma_rs::xz_compress(&mut &tar[..], &mut xz).unwrap();
        let xz_path = dir.path().join("n.tar.xz");
        fs::write(&xz_path, xz).unwrap();
        let out = dir.path().join("xz");
        extract(&xz_path, &ArchiveKind::TarXz, &out).unwrap();
        assert_eq!(fs::read(out.join("bin/node")).unwrap(), b"node");
        assert!(!dir.path().join("n.tar.xz.untar").exists());

        // gz single file
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut encoder, b"ra").unwrap();
        let gz_path = dir.path().join("ra.gz");
        fs::write(&gz_path, encoder.finish().unwrap()).unwrap();
        let out = dir.path().join("gz");
        let kind = ArchiveKind::Gz {
            file_name: "rust-analyzer".into(),
        };
        extract(&gz_path, &kind, &out).unwrap();
        assert_eq!(fs::read(out.join("rust-analyzer")).unwrap(), b"ra");

        let bad = ArchiveKind::Raw {
            file_name: "../x".into(),
        };
        assert!(extract(&gz_path, &bad, &out).is_err());
    }
}
