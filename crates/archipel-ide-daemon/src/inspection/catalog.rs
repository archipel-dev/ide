//! What can be configured: the registered inspections and intentions,
//! grouped for the settings page ("Editor > Inspections" / "Intentions").

use archipel_ide_analysis::inspect::{Level, LocalInspection, OptionDef};
use archipel_ide_analysis::intention::IntentionAction;
use archipel_ide_language::{LANGUAGES, ServiceRegistry};
use std::collections::BTreeMap;

/// One inspection, merged over the languages it is registered for.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static [&'static str],
    pub description: &'static str,
    pub default_level: Level,
    pub enabled_by_default: bool,
    pub languages: Vec<&'static str>,
    pub options: &'static [OptionDef],
}

/// All registered local inspections (sorted by group, then name).
pub fn inspections(registry: &ServiceRegistry) -> Vec<ToolInfo> {
    let mut by_id: BTreeMap<&'static str, ToolInfo> = BTreeMap::new();
    for lang in LANGUAGES.iter() {
        for tool in registry.all::<dyn LocalInspection>(lang.language_id()) {
            let entry = by_id.entry(tool.id()).or_insert_with(|| ToolInfo {
                id: tool.id(),
                name: tool.display_name(),
                group: tool.group_path(),
                description: tool.description_md(),
                default_level: tool.default_level(),
                enabled_by_default: tool.enabled_by_default(),
                languages: Vec::new(),
                options: tool.options(),
            });
            if !entry.languages.contains(&lang.id) {
                entry.languages.push(lang.id);
            }
        }
    }
    let mut tools: Vec<ToolInfo> = by_id.into_values().collect();
    tools.sort_by(|a, b| a.group.cmp(b.group).then(a.name.cmp(b.name)));
    tools
}

/// One intention for the intentions settings page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentionInfo {
    pub id: &'static str,
    pub family: &'static str,
}

/// All registered intentions (sorted by family, then id).
pub fn intentions(registry: &ServiceRegistry) -> Vec<IntentionInfo> {
    let mut out: BTreeMap<&'static str, IntentionInfo> = BTreeMap::new();
    for lang in LANGUAGES.iter() {
        for action in registry.all::<dyn IntentionAction>(lang.language_id()) {
            out.entry(action.id()).or_insert(IntentionInfo {
                id: action.id(),
                family: action.family(),
            });
        }
    }
    let mut list: Vec<_> = out.into_values().collect();
    list.sort_by(|a, b| a.family.cmp(b.family).then(a.id.cmp(b.id)));
    list
}

/// A group node of the settings tree.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupNode {
    pub name: &'static str,
    pub children: Vec<GroupNode>,
    pub tools: Vec<ToolInfo>,
}

/// Groups `tools` by their group path.
pub fn tree(tools: &[ToolInfo]) -> Vec<GroupNode> {
    fn insert(level: &mut Vec<GroupNode>, path: &[&'static str], tool: &ToolInfo) {
        let Some((first, rest)) = path.split_first() else {
            return;
        };
        let at = match level.iter().position(|g| g.name == *first) {
            Some(at) => at,
            None => {
                level.push(GroupNode {
                    name: first,
                    ..GroupNode::default()
                });
                level.len() - 1
            }
        };
        if rest.is_empty() {
            level[at].tools.push(tool.clone());
        } else {
            insert(&mut level[at].children, rest, tool);
        }
    }
    let mut roots = Vec::new();
    for tool in tools {
        insert(&mut roots, tool.group, tool);
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspection::tier_a;
    use archipel_ide_language::ServiceRegistry;

    #[test]
    fn catalog_merges_languages_and_builds_a_tree() {
        let mut registry = ServiceRegistry::new();
        tier_a::register(&mut registry);
        let tools = inspections(&registry);
        let todo = tools.iter().find(|t| t.id == "TodoComment").unwrap();
        assert!(todo.languages.len() > 10 && todo.languages.contains(&"rust"));
        let ids: Vec<_> = tools.iter().map(|t| t.id).collect();
        assert!(ids.windows(2).all(|w| w[0] != w[1]), "one entry per id");
        let groups = tree(&tools);
        let general = groups.iter().find(|g| g.name == "General").unwrap();
        assert!(
            general
                .children
                .iter()
                .any(|g| g.name == "Probable bugs" && !g.tools.is_empty())
        );
        let total: usize = groups
            .iter()
            .flat_map(|g| g.children.iter())
            .map(|c| c.tools.len())
            .sum();
        assert_eq!(total, tools.len());
    }
}
