//! A restricted Lua 5.4 state: no `io`, `os`, `package`, `debug`, `load`,
//! `dofile` or `loadfile`; `require` only loads `.lua` files inside the plugin
//! directory; memory is capped and every call runs under a time budget.

use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Value, VmState};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

pub const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;

/// Deadline of the running call, checked by the instruction hook.
#[derive(Clone, Default)]
pub struct Budget(Rc<Cell<Option<Instant>>>);

impl Budget {
    /// Runs `f` with a deadline of `limit` from now.
    pub fn run<R>(&self, limit: Duration, f: impl FnOnce() -> R) -> R {
        let previous = self.0.replace(Some(Instant::now() + limit));
        let result = f();
        self.0.set(previous);
        result
    }

    fn expired(&self) -> bool {
        self.0
            .get()
            .is_some_and(|deadline| Instant::now() > deadline)
    }
}

pub fn read_source(path: &Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(format!("{} is larger than 4 MiB", path.display()));
    }
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Creates the sandboxed state for the plugin in `dir`.
pub fn new_state(plugin_id: &str, dir: &Path, budget: &Budget) -> mlua::Result<Lua> {
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8 | StdLib::COROUTINE,
        LuaOptions::new(),
    )?;
    lua.set_memory_limit(MEMORY_LIMIT)?;
    let hook_budget = budget.clone();
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(1_000),
        move |_, _| {
            if hook_budget.expired() {
                Err(mlua::Error::runtime(
                    "script exceeded its time budget (endless loop?)",
                ))
            } else {
                Ok(VmState::Continue)
            }
        },
    );
    let globals = lua.globals();
    for name in ["dofile", "loadfile", "load"] {
        globals.set(name, Value::Nil)?;
    }
    let id = plugin_id.to_owned();
    globals.set(
        "print",
        lua.create_function(move |_, values: mlua::Variadic<Value>| {
            let text: Vec<String> = values
                .iter()
                .map(|value| value.to_string().unwrap_or_else(|_| format!("{value:?}")))
                .collect();
            eprintln!("[{id}] {}", text.join("\t"));
            Ok(())
        })?,
    )?;
    let root = dir.to_path_buf();
    let loaded = lua.create_table()?;
    globals.set(
        "require",
        lua.create_function(move |lua, name: String| {
            if let Ok(value) = loaded.get::<Value>(name.as_str())
                && !value.is_nil()
            {
                return Ok(value);
            }
            let path = module_path(&root, &name).map_err(mlua::Error::runtime)?;
            let source = read_source(&path).map_err(mlua::Error::runtime)?;
            let value: Value = lua
                .load(source)
                .set_name(format!("@{name}.lua"))
                .set_mode(mlua::ChunkMode::Text)
                .call(name.clone())?;
            let value = if value.is_nil() {
                Value::Boolean(true)
            } else {
                value
            };
            loaded.set(name, value.clone())?;
            Ok(value)
        })?,
    )?;
    let archipel = lua.create_table()?;
    archipel.set("api_version", archipel_ide_plugin_api::API_VERSION)?;
    globals.set("archipel", archipel.clone())?;
    // Compatibility: plugins written for the Jetthing versions use the global `jetthing`;
    // it is the same table as `archipel`.
    globals.set("jetthing", archipel)?;
    drop(globals);
    Ok(lua)
}

/// `a.b` → `<root>/a/b.lua`; only plain module names are accepted.
fn module_path(root: &Path, name: &str) -> Result<PathBuf, String> {
    let valid = !name.is_empty()
        && name.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        });
    if !valid {
        return Err(format!(
            "invalid module name `{name}` (use dotted names like `lib.util`)"
        ));
    }
    let mut path = root.to_path_buf();
    for part in name.split('.') {
        path.push(part);
    }
    path.set_extension("lua");
    if !path.is_file() {
        return Err(format!("module `{name}` not found at {}", path.display()));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_names_cannot_escape_the_plugin() {
        let root = Path::new("/plugins/x");
        for bad in ["", "../etc", "a..b", "/abs", "a/b", "a.\\b"] {
            assert!(module_path(root, bad).is_err(), "{bad}");
        }
    }
}
