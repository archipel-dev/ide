//! Plugins compiled into this build and the runtimes for script plugins.
//! First-party plugin crates live under `plugins/`; add one line per plugin
//! to [`bundled`].

use archipel_ide_ui::{PluginFactory, PluginSet};
use std::rc::Rc;

#[allow(clippy::vec_init_then_push)] // pushes depend on enabled features
pub fn bundled() -> Vec<PluginFactory> {
    #[allow(unused_mut)]
    let mut plugins: Vec<PluginFactory> = Vec::new();
    #[cfg(feature = "mongo-raabit")]
    plugins.push(Rc::new(|| Box::new(mongo_raabit::MongoRaabitPlugin::new())));
    #[cfg(feature = "nexus-http")]
    plugins.push(Rc::new(|| {
        Box::new(nexus_http::plugin::NexusHttpPlugin::new())
    }));
    #[cfg(feature = "example-plugins")]
    plugins.push(Rc::new(|| {
        Box::new(archipel_ide_plugin_hello::HelloPlugin::new())
    }));
    plugins
}

pub fn plugin_set() -> PluginSet {
    let mut loaders: Vec<Rc<dyn archipel_ide_ui::PluginLoader>> =
        vec![Rc::new(archipel_ide_plugin_lua::LuaLoader)];
    match archipel_ide_plugin_wasm::WasmLoader::new() {
        Ok(loader) => loaders.push(Rc::new(loader)),
        // WASM plugins then show as unsupported with this reason in the log.
        Err(error) => eprintln!("archipel-ide: WASM plugin runtime unavailable: {error}"),
    }
    PluginSet {
        bundled: bundled(),
        loaders,
    }
}

#[cfg(all(test, feature = "nexus-http"))]
mod tests {
    use super::*;

    #[test]
    fn nexus_http_is_bundled_with_a_tools_menu_entry() {
        let plugin = bundled()
            .iter()
            .map(|factory| factory())
            .find(|plugin| plugin.manifest().id == "dev.archipel.nexus-http")
            .expect("nexus-http is bundled by default");
        let contributions = plugin.contributions();
        assert!(contributions.commands.iter().any(|c| c.id == "nexus.open"));
        assert!(
            contributions
                .menus
                .iter()
                .any(|m| m.command == "nexus.open")
        );
        assert!(plugin.manifest().app.is_some());
    }
}
