//! Shared helpers of the integration tests.
#![allow(dead_code)]

use archipel_ide_analysis::index::IndexStore;
use archipel_ide_analysis::vfs::DiskVfs;
use archipel_ide_analysis::{AnalysisHost, HostConfig, IndexId, default_registry};
use archipel_ide_index::{IndexConfig, IndexService};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub fn host() -> AnalysisHost {
    AnalysisHost::new(HostConfig::testing(), Arc::new(DiskVfs), default_registry())
}

pub fn service(store_dir: Option<&Path>) -> IndexService {
    let mut cfg = IndexConfig::testing();
    cfg.store_dir = store_dir.map(Path::to_path_buf);
    IndexService::new(cfg)
}

pub fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    std::fs::write(path, text).expect("write");
}

/// Attaches `service` to `root` and runs it to completion.
pub fn index(service: &mut IndexService, host: &mut AnalysisHost, root: &Path) {
    service.attach(host, root);
    assert!(
        service.run_until_idle(host, Duration::from_secs(60)),
        "indexing finished"
    );
}

pub fn settle(service: &mut IndexService, host: &mut AnalysisHost) {
    assert!(
        service.run_until_idle(host, Duration::from_secs(60)),
        "indexing finished"
    );
}

/// `(index, key) -> (path -> value)` of every index, with file ids mapped to
/// paths (so two runs with different ids compare equal).
pub type Dump = BTreeMap<(String, Vec<u8>), BTreeMap<String, Vec<u8>>>;

pub fn dump(service: &IndexService, host: &AnalysisHost) -> Dump {
    let snapshot = host.snapshot();
    let overlay = service.overlay().expect("attached");
    let mut out = Dump::new();
    for index in service.indexes().all() {
        let id: IndexId = index.id();
        for key in overlay.keys(id) {
            let files = overlay.get(id, &key);
            for (file, value) in files {
                let path = snapshot.path(file).map_or_else(
                    || format!("<retired {}>", file.0),
                    |p| p.display().to_string(),
                );
                let key_name = if id.0 == "Stubs" {
                    path.clone().into_bytes()
                } else {
                    key.clone()
                };
                out.entry((id.0.to_string(), key_name))
                    .or_default()
                    .insert(path, value);
            }
        }
    }
    out
}
