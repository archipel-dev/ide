//! Local plugin registry: a directory of `.archipelplugin` archives that the
//! Marketplace tab lists. Entries are read from the archives' own manifests, so
//! nothing is shown that cannot be installed, and no network is involved.

use crate::{PluginManifest, archive};
use std::fs;
use std::path::{Path, PathBuf};

/// One installable plugin offered by a registry directory.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub manifest: PluginManifest,
    /// The `.archipelplugin` archive `plugin.install` installs from.
    pub archive: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
    /// Archives that could not be read, with the reason.
    pub problems: Vec<(PathBuf, String)>,
}

impl Catalog {
    /// Reads every `*.archipelplugin` in `dirs`. The first archive per plugin id wins
    /// (directories are in priority order); entries are sorted by name.
    pub fn scan(dirs: &[PathBuf]) -> Self {
        let mut catalog = Self::default();
        for dir in dirs {
            catalog.scan_dir(dir);
        }
        catalog
            .entries
            .sort_by_key(|entry| entry.manifest.name.to_lowercase());
        catalog
    }

    fn scan_dir(&mut self, dir: &Path) {
        let Ok(read) = fs::read_dir(dir) else {
            return; // a registry directory that does not exist is simply empty
        };
        let mut archives: Vec<PathBuf> = read
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && archive::is_archive(path))
            .collect();
        archives.sort();
        for path in archives {
            match archive::read_manifest(&path) {
                Ok(manifest) => {
                    if self.entries.iter().all(|e| e.manifest.id != manifest.id) {
                        self.entries.push(CatalogEntry {
                            manifest,
                            archive: path,
                        });
                    }
                }
                Err(error) => self.problems.push((path, error.to_string())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_packed_plugins_and_reports_broken_archives() {
        let root = std::env::temp_dir().join(format!("jt-catalog-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let plugin = root.join("src/hello");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(
            plugin.join("archipel-plugin.toml"),
            "id='dev.test.hello'\nname='Hello'\nversion='1.0.0'\napi_version=1\nkind='lua'\nentry='main.lua'\nvendor='Test'\n",
        )
        .unwrap();
        fs::write(plugin.join("main.lua"), "").unwrap();
        let registry = root.join("registry");
        fs::create_dir_all(&registry).unwrap();
        archive::pack(&plugin, &registry.join("hello.archipelplugin")).unwrap();
        fs::write(registry.join("broken.archipelplugin"), "not a zip").unwrap();
        // Archives from before the rename end in `.jtplugin` and are still listed.
        let old = root.join("src/old");
        fs::create_dir_all(&old).unwrap();
        fs::write(
            old.join("archipel-plugin.toml"),
            "id='dev.test.old'\nname='Old'\nversion='1.0.0'\napi_version=1\nkind='lua'\nentry='main.lua'\n",
        )
        .unwrap();
        fs::write(old.join("main.lua"), "").unwrap();
        archive::pack(&old, &registry.join("old.jtplugin")).unwrap();
        let catalog = Catalog::scan(&[root.join("missing"), registry.clone()]);
        let ids: Vec<&str> = catalog
            .entries
            .iter()
            .map(|entry| entry.manifest.id.as_str())
            .collect();
        assert_eq!(ids, ["dev.test.hello", "dev.test.old"]);
        assert_eq!(catalog.problems.len(), 1);
        let _ = fs::remove_dir_all(&root);
    }
}
