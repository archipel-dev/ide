//! Build variants (`buildTypes` × `productFlavors`) read statically from a
//! module's Gradle script, as shown in the Build Variants tool window.
//!
//! Scripts are not evaluated: build types and flavors declared through loops
//! or plugins are not seen, and `debug`/`release` always exist.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flavor {
    pub name: String,
    pub dimension: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    /// e.g. `freeStagingDebug`.
    pub name: String,
    /// Flavors in dimension order.
    pub flavors: Vec<String>,
    pub build_type: String,
}

impl Variant {
    /// `assemble<Variant>` task name suffix, e.g. `FreeDebug`.
    pub fn capitalized(&self) -> String {
        capitalize(&self.name)
    }

    /// APK output directory relative to the module, e.g.
    /// `build/outputs/apk/free/debug` (AGP layout: flavors joined, then build type).
    pub fn apk_dir(&self) -> String {
        if self.flavors.is_empty() {
            format!("build/outputs/apk/{}", self.build_type)
        } else {
            let flavor = lower_first(
                &self
                    .flavors
                    .iter()
                    .map(|f| capitalize(f))
                    .collect::<String>(),
            );
            format!("build/outputs/apk/{flavor}/{}", self.build_type)
        }
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Body of the first `name { … }` block (brace-matched), if any.
fn block<'a>(script: &'a str, name: &str) -> Option<&'a str> {
    let mut search = 0;
    while let Some(found) = script[search..].find(name) {
        let start = search + found;
        search = start + name.len();
        let before_ok = script[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.'));
        let rest = script[search..].trim_start();
        if !before_ok || !rest.starts_with('{') {
            continue;
        }
        let open = script.len() - rest.len();
        let mut depth = 0usize;
        for (offset, ch) in script[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&script[open + 1..open + offset]);
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    None
}

/// Named children of a container block: `name {`, `create("name") {`,
/// `getByName("name")`, `register("name")`, `maybeCreate("name")`.
fn children(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut statement = String::new();
    let mut chars = body.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '{' => {
                if depth == 0 {
                    let header = statement.trim().to_owned();
                    // Find the matching close brace for the child body.
                    let mut inner_depth = 1usize;
                    let start = index + 1;
                    let mut end = body.len();
                    for (j, c) in body[start..].char_indices() {
                        match c {
                            '{' => inner_depth += 1,
                            '}' => {
                                inner_depth -= 1;
                                if inner_depth == 0 {
                                    end = start + j;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(name) = child_name(&header) {
                        out.push((name, body[start..end].to_owned()));
                    }
                    while chars.peek().is_some_and(|(i, _)| *i <= end) {
                        chars.next();
                    }
                    statement.clear();
                    continue;
                }
                depth += 1;
            }
            '\n' | ';' if depth == 0 => {
                if let Some(name) = child_name(statement.trim()).filter(|_| statement.contains('('))
                {
                    out.push((name, String::new()));
                }
                statement.clear();
            }
            _ => statement.push(ch),
        }
    }
    if let Some(name) = child_name(statement.trim()).filter(|_| statement.contains('(')) {
        out.push((name, String::new()));
    }
    out
}

fn child_name(header: &str) -> Option<String> {
    let header = header.trim();
    for call in ["create", "getByName", "register", "maybeCreate", "named"] {
        if let Some(rest) = header.strip_prefix(call) {
            let rest = rest.trim_start().strip_prefix('(')?;
            let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let name: String = rest[1..].chars().take_while(|c| *c != quote).collect();
            return (!name.is_empty()).then_some(name);
        }
    }
    let ident = header.trim_end_matches(|c: char| c.is_whitespace());
    (!ident.is_empty() && ident.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .then(|| ident.to_owned())
}

fn quoted(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' {
            out.push(chars.by_ref().take_while(|c| *c != ch).collect());
        }
    }
    out
}

/// Build types, flavors and variants of a module script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantModel {
    pub build_types: Vec<String>,
    pub dimensions: Vec<String>,
    pub flavors: Vec<Flavor>,
    pub variants: Vec<Variant>,
}

impl VariantModel {
    pub fn parse(script: &str) -> Self {
        let android = block(script, "android").unwrap_or(script);
        let mut build_types = vec!["debug".to_owned(), "release".to_owned()];
        if let Some(body) = block(android, "buildTypes") {
            for (name, _) in children(body) {
                if !build_types.contains(&name) {
                    build_types.push(name);
                }
            }
        }
        let mut dimensions: Vec<String> = android
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("flavorDimensions"))
            .flat_map(quoted)
            .collect();
        let flavors: Vec<Flavor> = block(android, "productFlavors")
            .map(|body| {
                children(body)
                    .into_iter()
                    .map(|(name, body)| Flavor {
                        dimension: body
                            .lines()
                            .map(str::trim)
                            .find(|l| l.starts_with("dimension"))
                            .and_then(|l| quoted(l).into_iter().next()),
                        name,
                    })
                    .collect()
            })
            .unwrap_or_default();
        for flavor in &flavors {
            if let Some(dimension) = &flavor.dimension
                && !dimensions.contains(dimension)
            {
                dimensions.push(dimension.clone());
            }
        }
        // Group flavors by dimension (flavors without one form a single group).
        let mut groups: Vec<Vec<&Flavor>> = dimensions
            .iter()
            .map(|d| {
                flavors
                    .iter()
                    .filter(|f| f.dimension.as_ref() == Some(d))
                    .collect()
            })
            .filter(|g: &Vec<&Flavor>| !g.is_empty())
            .collect();
        let undimensioned: Vec<&Flavor> =
            flavors.iter().filter(|f| f.dimension.is_none()).collect();
        if !undimensioned.is_empty() {
            groups.push(undimensioned);
        }
        let mut combos: Vec<Vec<String>> = vec![Vec::new()];
        for group in &groups {
            combos = combos
                .iter()
                .flat_map(|combo| {
                    group.iter().map(move |flavor| {
                        let mut next = combo.clone();
                        next.push(flavor.name.clone());
                        next
                    })
                })
                .collect();
        }
        let mut variants = Vec::new();
        for combo in &combos {
            for build_type in &build_types {
                let name = if combo.is_empty() {
                    build_type.clone()
                } else {
                    let mut name = combo[0].clone();
                    for part in combo[1..].iter().chain(std::iter::once(build_type)) {
                        name.push_str(&capitalize(part));
                    }
                    name
                };
                variants.push(Variant {
                    name,
                    flavors: combo.clone(),
                    build_type: build_type.clone(),
                });
            }
        }
        Self {
            build_types,
            dimensions,
            flavors,
            variants,
        }
    }

    /// The default variant, like the reference IDE: the first debug variant.
    pub fn default_variant(&self) -> Option<&Variant> {
        self.variants
            .iter()
            .find(|v| v.build_type == "debug")
            .or(self.variants.first())
    }

    pub fn find(&self, name: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_module_has_debug_and_release() {
        let model = VariantModel::parse("android { namespace = \"x\" }");
        let names: Vec<_> = model.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["debug", "release"]);
        assert_eq!(
            model.default_variant().unwrap().apk_dir(),
            "build/outputs/apk/debug"
        );
        assert_eq!(model.default_variant().unwrap().capitalized(), "Debug");
    }

    #[test]
    fn kotlin_dsl_flavors_and_custom_build_types() {
        let script = r#"
android {
    namespace = "com.example"
    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
        }
        create("staging") {
            initWith(getByName("debug"))
        }
    }
    flavorDimensions += listOf("tier", "store")
    productFlavors {
        create("free") { dimension = "tier" }
        create("paid") {
            dimension = "tier"
            applicationIdSuffix = ".paid"
        }
        create("play") { dimension = "store" }
    }
}
"#;
        let model = VariantModel::parse(script);
        assert_eq!(model.build_types, ["debug", "release", "staging"]);
        assert_eq!(model.dimensions, ["tier", "store"]);
        let names: Vec<_> = model.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "freePlayDebug",
                "freePlayRelease",
                "freePlayStaging",
                "paidPlayDebug",
                "paidPlayRelease",
                "paidPlayStaging"
            ]
        );
        let variant = model.find("paidPlayDebug").unwrap();
        assert_eq!(variant.capitalized(), "PaidPlayDebug");
        assert_eq!(variant.apk_dir(), "build/outputs/apk/paidPlay/debug");
        assert_eq!(model.default_variant().unwrap().name, "freePlayDebug");
    }

    #[test]
    fn groovy_flavors() {
        let script = "android {\n  flavorDimensions 'mode'\n  productFlavors {\n    demo {\n      dimension 'mode'\n    }\n    full {\n      dimension 'mode'\n    }\n  }\n  buildTypes {\n    benchmark { }\n  }\n}\n";
        let model = VariantModel::parse(script);
        let names: Vec<_> = model.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "demoDebug",
                "demoRelease",
                "demoBenchmark",
                "fullDebug",
                "fullRelease",
                "fullBenchmark"
            ]
        );
    }

    #[test]
    fn malformed_blocks_do_not_panic() {
        for script in [
            "android {",
            "android { buildTypes { create(\"x\"",
            "productFlavors {{{",
            "}}}",
        ] {
            let model = VariantModel::parse(script);
            assert!(model.variants.iter().any(|v| v.name == "debug"));
        }
    }
}
