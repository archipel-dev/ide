//! The Spring Boot starter catalog behind the New Project wizard.
//!
//! The built-in catalog (`data/spring-catalog.json`) is the default and works fully
//! offline. [`fetch_remote`] can refresh Boot versions and add starters from a
//! start.spring.io compatible service; the wizard keeps working when it is unreachable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BUILTIN: &str = include_str!("../data/spring-catalog.json");

/// Metadata service the optional refresh talks to.
pub const INITIALIZR_URL: &str = "https://start.spring.io";
/// Accept header of the Initializr metadata API.
pub const INITIALIZR_ACCEPT: &str = "application/vnd.initializr.v2.2+json";

/// Build configuration a coordinate is added to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    Implementation,
    CompileOnly,
    DevelopmentOnly,
    RuntimeOnly,
    AnnotationProcessor,
    /// `testImplementation` in Gradle, `<scope>test</scope>` in Maven.
    Test,
    TestCompileOnly,
    TestAnnotationProcessor,
}

/// One Maven artifact of a starter. Versions are left out where Spring Boot manages them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coordinate {
    pub group: String,
    pub artifact: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<Scope>,
    /// Only added when the dependency with this id is selected too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Annotation processor class, for Bazel `java_plugin` rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor: Option<String>,
}

fn default_scopes() -> Vec<Scope> {
    vec![Scope::Implementation]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub coordinates: Vec<Coordinate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub version: u32,
    /// Newest first; the first entry is the default.
    pub boot_versions: Vec<String>,
    /// Kotlin version Spring Boot manages, by Boot `major.minor` line (`"4.1": "2.3.21"`).
    #[serde(default)]
    pub kotlin_versions: BTreeMap<String, String>,
    pub java_versions: Vec<u32>,
    pub categories: Vec<Category>,
    pub dependencies: Vec<Dependency>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self::builtin()
    }
}

impl Catalog {
    /// The catalog shipped with the IDE. An unreadable bundled file (a build bug, covered
    /// by a test) yields an empty catalog instead of a panic.
    pub fn builtin() -> Self {
        serde_json::from_str(BUILTIN).unwrap_or(Self {
            version: 0,
            boot_versions: Vec::new(),
            kotlin_versions: BTreeMap::new(),
            java_versions: vec![17, 21, 25],
            categories: Vec::new(),
            dependencies: Vec::new(),
        })
    }

    /// Kotlin version for a Boot version: the one of its `major.minor` line, else of the
    /// newest line the catalog knows.
    pub fn kotlin_version(&self, boot_version: &str) -> String {
        let line = boot_version
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".");
        let number = |key: &str| -> Vec<u32> {
            key.split('.')
                .filter_map(|part| part.parse().ok())
                .collect()
        };
        self.kotlin_versions
            .get(&line)
            .or_else(|| {
                self.kotlin_versions
                    .iter()
                    .max_by_key(|(key, _)| number(key))
                    .map(|(_, v)| v)
            })
            .cloned()
            .unwrap_or_else(|| "2.3.21".to_owned())
    }

    pub fn dependency(&self, id: &str) -> Option<&Dependency> {
        self.dependencies
            .iter()
            .find(|dependency| dependency.id == id)
    }

    pub fn category_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.categories
            .iter()
            .find(|category| category.id == id)
            .map_or(id, |category| category.name.as_str())
    }

    /// Dependencies matching `query` (id, name or description, case-insensitive; every
    /// word must match) in `category` (all when `None`), in catalog order.
    pub fn search(&self, query: &str, category: Option<&str>) -> Vec<&Dependency> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.dependencies
            .iter()
            .filter(|dependency| category.is_none_or(|id| dependency.category == id))
            .filter(|dependency| {
                let haystack = format!(
                    "{} {} {}",
                    dependency.id, dependency.name, dependency.description
                )
                .to_lowercase();
                words.iter().all(|word| haystack.contains(word))
            })
            .collect()
    }

    /// Merges a start.spring.io refresh: replaces the Boot versions (Boot 4.x releases
    /// only, the line the bundled coordinates are written for) and adds starters that
    /// the catalog lacks. Existing entries keep their own descriptions. Returns the
    /// number of added starters.
    pub fn merge_remote(&mut self, remote: &Remote) -> usize {
        let versions: Vec<String> = remote
            .boot_versions
            .iter()
            .filter(|version| supported_boot_version(version))
            .cloned()
            .collect();
        if !versions.is_empty() {
            self.boot_versions = versions;
        }
        let mut added = 0;
        for entry in &remote.dependencies {
            if self.dependency(&entry.id).is_some() || entry.coordinates.is_empty() {
                continue;
            }
            let category = slug(&entry.group);
            if !self
                .categories
                .iter()
                .any(|existing| existing.id == category)
            {
                self.categories.push(Category {
                    id: category.clone(),
                    name: entry.group.clone(),
                });
            }
            self.dependencies.push(Dependency {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                category,
                coordinates: entry.coordinates.clone(),
            });
            added += 1;
        }
        added
    }
}

/// `4.1.2` is supported; snapshots, milestones and other major lines are not (the
/// bundled coordinates use the modular starters of Spring Boot 4).
pub fn supported_boot_version(version: &str) -> bool {
    let mut parts = version.split('.');
    parts.next() == Some("4")
        && parts.clone().count() == 2
        && parts.all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

/// A starter reported by the metadata service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDependency {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Initializr group name, e.g. `Web`.
    pub group: String,
    /// Empty when the coordinates are unknown or need a BOM or repository.
    pub coordinates: Vec<Coordinate>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Remote {
    pub boot_versions: Vec<String>,
    pub dependencies: Vec<RemoteDependency>,
}

/// Parses the root metadata document (`GET /`, [`INITIALIZR_ACCEPT`]).
pub fn parse_metadata(json: &str) -> Result<Remote, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| format!("invalid metadata: {error}"))?;
    let text = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let boot_versions = value["bootVersion"]["values"]
        .as_array()
        .map(|values| values.iter().map(|v| text(v, "id")).collect())
        .unwrap_or_default();
    let mut dependencies = Vec::new();
    for group in value["dependencies"]["values"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let group_name = text(group, "name");
        for entry in group["values"].as_array().into_iter().flatten() {
            let id = text(entry, "id");
            if id.is_empty() {
                continue;
            }
            dependencies.push(RemoteDependency {
                name: text(entry, "name"),
                description: text(entry, "description"),
                group: group_name.clone(),
                id,
                coordinates: Vec::new(),
            });
        }
    }
    if dependencies.is_empty() {
        return Err("the service returned no dependencies".into());
    }
    Ok(Remote {
        boot_versions,
        dependencies,
    })
}

/// Parses the coordinates document (`GET /dependencies?bootVersion=…`) into `remote`.
/// Entries that need a BOM or an extra repository are skipped: the generators only
/// write plain, resolvable coordinates.
pub fn parse_coordinates(json: &str, remote: &mut Remote) -> Result<(), String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| format!("invalid coordinates: {error}"))?;
    let map: BTreeMap<String, serde_json::Value> = value["dependencies"]
        .as_object()
        .ok_or("the service returned no coordinates")?
        .clone()
        .into_iter()
        .collect();
    for dependency in &mut remote.dependencies {
        let Some(entry) = map.get(&dependency.id) else {
            continue;
        };
        if entry.get("bom").is_some() || entry.get("repository").is_some() {
            continue;
        }
        let (Some(group), Some(artifact)) = (
            entry.get("groupId").and_then(serde_json::Value::as_str),
            entry.get("artifactId").and_then(serde_json::Value::as_str),
        ) else {
            continue;
        };
        let scope = match entry.get("scope").and_then(serde_json::Value::as_str) {
            None | Some("compile") => Scope::Implementation,
            Some("runtime") => Scope::RuntimeOnly,
            Some("provided") => Scope::CompileOnly,
            Some("annotationProcessor") => Scope::AnnotationProcessor,
            Some("test") => Scope::Test,
            Some(_) => continue,
        };
        dependency.coordinates = vec![Coordinate {
            group: group.to_owned(),
            artifact: artifact.to_owned(),
            version: entry
                .get("version")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            scopes: vec![scope],
            when: None,
            processor: None,
        }];
    }
    Ok(())
}

/// Fetches Boot versions and starters from `base` (normally [`INITIALIZR_URL`]).
/// Blocking: call it from a worker thread. Errors are phrased for the wizard.
pub fn fetch_remote(base: &str) -> Result<Remote, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();
    let get = |path: &str| -> Result<String, String> {
        agent
            .get(&format!("{}{path}", base.trim_end_matches('/')))
            .set("Accept", INITIALIZR_ACCEPT)
            .call()
            .map_err(|error| format!("cannot reach {base}: {error}"))?
            .into_string()
            .map_err(|error| format!("cannot read the response of {base}: {error}"))
    };
    let mut remote = parse_metadata(&get("/")?)?;
    let newest = remote
        .boot_versions
        .iter()
        .find(|version| supported_boot_version(version))
        .cloned();
    if let Some(version) = newest {
        parse_coordinates(
            &get(&format!("/dependencies?bootVersion={version}"))?,
            &mut remote,
        )?;
    }
    Ok(remote)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_catalog_is_consistent() {
        let catalog = Catalog::builtin();
        assert!(catalog.dependencies.len() >= 40);
        assert_eq!(catalog.kotlin_version("4.1.9"), "2.3.21");
        assert_eq!(catalog.kotlin_version("4.0.1"), "2.2.21");
        assert_eq!(
            catalog.kotlin_version("4.9.0"),
            "2.3.21",
            "unknown lines use the newest"
        );
        assert!(
            catalog
                .boot_versions
                .iter()
                .all(|v| supported_boot_version(v))
        );
        let mut ids = std::collections::BTreeSet::new();
        for dependency in &catalog.dependencies {
            assert!(ids.insert(&dependency.id), "duplicate id {}", dependency.id);
            assert!(!dependency.description.is_empty(), "{}", dependency.id);
            assert!(!dependency.coordinates.is_empty(), "{}", dependency.id);
            assert!(
                catalog
                    .categories
                    .iter()
                    .any(|c| c.id == dependency.category),
                "unknown category of {}",
                dependency.id
            );
            for coordinate in &dependency.coordinates {
                assert!(!coordinate.group.is_empty() && !coordinate.artifact.is_empty());
                if let Some(when) = &coordinate.when {
                    assert!(catalog.dependency(when).is_some(), "{when} is unknown");
                }
            }
        }
        for id in [
            "web",
            "webflux",
            "data-jpa",
            "jdbc",
            "security",
            "oauth2-client",
            "oauth2-resource-server",
            "rabbitmq",
            "validation",
            "actuator",
            "devtools",
            "lombok",
            "configuration-processor",
            "thymeleaf",
            "mustache",
            "postgresql",
            "mysql",
            "h2",
            "mariadb",
            "data-mongodb",
            "data-redis",
            "kafka",
            "rabbitmq",
            "mail",
            "cache",
            "batch",
            "quartz",
            "docker-compose",
            "testcontainers",
            "graphql",
            "websocket",
            "flyway",
            "liquibase",
            "prometheus",
        ] {
            assert!(catalog.dependency(id).is_some(), "{id} missing");
        }
    }

    #[test]
    fn search_matches_words_and_filters_by_category() {
        let catalog = Catalog::builtin();
        let hits = catalog.search("postgres", None);
        assert!(hits.iter().any(|d| d.id == "postgresql"));
        let hits = catalog.search("rest json", None);
        assert!(hits.iter().all(|d| {
            let text = format!("{} {} {}", d.id, d.name, d.description).to_lowercase();
            text.contains("rest") && text.contains("json")
        }));
        assert!(
            catalog
                .search("", Some("nosql"))
                .iter()
                .all(|d| d.category == "nosql")
        );
        assert!(catalog.search("zzzz-no-such", None).is_empty());
    }

    #[test]
    fn remote_refresh_replaces_versions_and_adds_missing_starters() {
        let metadata = r#"{
          "bootVersion": {"default": "4.1.3", "values": [
            {"id": "5.0.0", "name": "5.0.0"}, {"id": "4.1.3", "name": "4.1.3"},
            {"id": "4.2.0-SNAPSHOT", "name": "snap"}, {"id": "4.0.10", "name": "4.0.10"}]},
          "dependencies": {"values": [
            {"name": "Web", "values": [
              {"id": "web", "name": "Spring Web", "description": "remote text"},
              {"id": "brand-new", "name": "Brand New", "description": "A new starter"}]},
            {"name": "AI", "values": [{"id": "needs-bom", "name": "Needs BOM", "description": "x"}]}]}
        }"#;
        let coordinates = r#"{"dependencies": {
          "web": {"groupId": "org.springframework.boot", "artifactId": "spring-boot-starter-web"},
          "brand-new": {"groupId": "com.example", "artifactId": "brand-new-starter", "version": "1.2.3", "scope": "runtime"},
          "needs-bom": {"groupId": "x", "artifactId": "y", "bom": "some-bom"}}}"#;
        let mut remote = parse_metadata(metadata).unwrap();
        parse_coordinates(coordinates, &mut remote).unwrap();
        let mut catalog = Catalog::builtin();
        let before = catalog.dependencies.len();
        let web_description = catalog.dependency("web").unwrap().description.clone();
        assert_eq!(catalog.merge_remote(&remote), 1);
        assert_eq!(catalog.dependencies.len(), before + 1);
        assert_eq!(catalog.boot_versions, ["4.1.3", "4.0.10"]);
        assert_eq!(
            catalog.dependency("web").unwrap().description,
            web_description
        );
        let added = catalog.dependency("brand-new").unwrap();
        assert_eq!(added.coordinates[0].version.as_deref(), Some("1.2.3"));
        assert_eq!(added.coordinates[0].scopes, [Scope::RuntimeOnly]);
        assert!(catalog.dependency("needs-bom").is_none());
    }

    #[test]
    fn broken_remote_documents_are_errors_not_panics() {
        assert!(parse_metadata("not json").is_err());
        assert!(parse_metadata("{}").is_err());
        let mut remote = Remote::default();
        assert!(parse_coordinates("[]", &mut remote).is_err());
        assert!(fetch_remote("http://127.0.0.1:1").is_err());
    }
}
