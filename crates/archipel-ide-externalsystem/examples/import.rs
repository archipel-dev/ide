//! `cargo run -p archipel-ide-externalsystem --example import -- <project dir> [class...]`
//!
//! Imports a project with every build system that applies and prints modules, roots,
//! dependencies and (for the class names given) the library that provides each class.

use archipel_ide_externalsystem::jar::provider_of;
use archipel_ide_externalsystem::{
    ExternalSystemRegistry, ProgressListener, ProjectDataManager, ResolveRequest, import_project,
};
use archipel_ide_projectmodel::{ClasspathKind, OrderEntry};
use std::sync::Arc;

struct Print;
impl ProgressListener for Print {
    fn text(&self, text: &str) {
        eprintln!("... {text}");
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next() else {
        eprintln!("usage: import <project dir> [qualified class name...]");
        std::process::exit(2);
    };
    let classes: Vec<String> = args.collect();
    let root = std::fs::canonicalize(&root).unwrap_or_else(|e| {
        eprintln!("{root}: {e}");
        std::process::exit(2);
    });
    let mut request = ResolveRequest::new(&root);
    request.listener = Arc::new(Print);
    let started = std::time::Instant::now();
    let result = match import_project(
        &ExternalSystemRegistry::with_defaults(),
        &ProjectDataManager::new(),
        &request,
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("import failed: {error}");
            std::process::exit(1);
        }
    };
    let model = &result.model;
    println!(
        "{} ({:?}) in {:.1}s: {} modules, {} libraries, {} nodes",
        model.name,
        result.systems,
        started.elapsed().as_secs_f32(),
        model.modules.len(),
        model.libraries.len(),
        result.nodes
    );
    for sdk in &model.sdks {
        println!("sdk {} {}", sdk.name, sdk.home.display());
    }
    for diagnostic in &result.diagnostics {
        println!("{:?}: {}", diagnostic.severity, diagnostic.message);
    }
    for module in &model.modules {
        println!("\nmodule {} ({:?})", module.name, module.module_type);
        for folder in module.source_folders() {
            println!(
                "  {:<14} {}",
                folder.root_type.id(),
                folder
                    .path
                    .strip_prefix(&root)
                    .unwrap_or(&folder.path)
                    .display()
            );
        }
        for path in module.excluded_folders() {
            println!(
                "  {:<14} {}",
                "excluded",
                path.strip_prefix(&root).unwrap_or(path).display()
            );
        }
        let (mut modules, mut libraries) = (Vec::new(), 0);
        for entry in &module.order_entries {
            match entry {
                OrderEntry::Module { module, scope, .. } => {
                    modules.push(format!("{module} ({})", scope.id()))
                }
                OrderEntry::Library { .. } => libraries += 1,
                _ => {}
            }
        }
        println!("  depends on modules: {modules:?}; {libraries} library entries");
        for class in &classes {
            for kind in [ClasspathKind::ProductionCompile, ClasspathKind::TestCompile] {
                if let Some(library) = provider_of(model, module, kind, class) {
                    println!("  {class} ({kind:?}) <- {}", library.name);
                }
            }
        }
    }
}
