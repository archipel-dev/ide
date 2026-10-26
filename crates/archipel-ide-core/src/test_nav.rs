//! Go to Test / Test Subject (Ctrl+Shift+T / ⇧⌘T) and Create Test for Java
//! and Kotlin sources.
//!
//! Pure path logic over the project tree, no language server: `Foo.java` in
//! `src/main/java/p` pairs with `FooTest`, `FooTests`, `TestFoo` or `FooIT` in
//! `src/test/java/p` (or `src/test/kotlin/p`), and the other way round. When a
//! source has no test yet, the command creates one next to the conventional
//! location with the package of the source.
use crate::navigation::{NavigationItem, NavigationKind, NavigationList};
use crate::{IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

const LANGUAGE_DIRS: [&str; 2] = ["java", "kotlin"];
const SUFFIXES: [&str; 3] = ["Test", "Tests", "IT"];

fn is_jvm_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("java" | "kt")
    )
}

/// The test class name a source class `name` gets when created.
fn created_test_name(name: &str) -> String {
    format!("{name}Test")
}

/// The class a test class name tests, if the name follows a test convention.
fn subject_name(test: &str) -> Option<String> {
    for suffix in SUFFIXES {
        if let Some(name) = test.strip_suffix(suffix).filter(|name| !name.is_empty()) {
            return Some(name.to_owned());
        }
    }
    test.strip_prefix("Test")
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .map(str::to_owned)
}

/// Splits `src/<set>/<lang>/<package dirs>/File.ext` into its parts.
struct SourceLocation {
    /// Components before `src`.
    base: PathBuf,
    set: String,
    language: String,
    package: PathBuf,
}

fn locate(path: &Path) -> Option<SourceLocation> {
    let parts: Vec<&str> = path
        .parent()?
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    let src = parts.iter().rposition(|part| *part == "src")?;
    let set = *parts.get(src + 1)?;
    let language = *parts.get(src + 2)?;
    if !LANGUAGE_DIRS.contains(&language) {
        return None;
    }
    Some(SourceLocation {
        base: parts[..src].iter().collect(),
        set: set.to_owned(),
        language: language.to_owned(),
        package: parts[src + 3..].iter().collect(),
    })
}

fn is_test_set(set: &str) -> bool {
    set == "test" || set.ends_with("Test")
}

/// Candidate paths of the tests of `path`, or of the sources a test tests.
/// `exists` filters afterwards; the first entry is where Create Test writes.
pub(crate) fn counterparts(path: &Path) -> Vec<PathBuf> {
    let (Some(stem), Some(extension)) = (
        path.file_stem().and_then(|s| s.to_str()),
        path.extension().and_then(|e| e.to_str()),
    ) else {
        return Vec::new();
    };
    let directory = path.parent().unwrap_or(Path::new(""));
    let located = locate(path);
    let in_test = match &located {
        Some(location) => is_test_set(&location.set),
        None => subject_name(stem).is_some(),
    };
    let mut candidates = Vec::new();
    let mut push = |directory: PathBuf, name: &str, extension: &str| {
        let candidate = directory.join(format!("{name}.{extension}"));
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };
    if in_test {
        let Some(subject) = subject_name(stem) else {
            return Vec::new();
        };
        match &located {
            Some(location) => {
                for language in LANGUAGE_DIRS {
                    let directory = location
                        .base
                        .join("src")
                        .join("main")
                        .join(language)
                        .join(&location.package);
                    push(directory.clone(), &subject, extension);
                    for other in ["java", "kt"] {
                        push(directory.clone(), &subject, other);
                    }
                }
            }
            None => {
                for other in ["java", "kt"] {
                    push(directory.to_owned(), &subject, other);
                }
            }
        }
        return candidates;
    }
    let names: Vec<String> = SUFFIXES
        .iter()
        .map(|suffix| format!("{stem}{suffix}"))
        .chain([format!("Test{stem}")])
        .collect();
    // The created test comes first; the first language directory is the
    // source's own.
    let mut directories = Vec::new();
    match &located {
        Some(location) => {
            let mut languages = vec![location.language.as_str()];
            languages.extend(
                LANGUAGE_DIRS
                    .into_iter()
                    .filter(|l| *l != location.language),
            );
            for language in languages {
                directories.push(
                    location
                        .base
                        .join("src")
                        .join("test")
                        .join(language)
                        .join(&location.package),
                );
            }
        }
        None => directories.push(directory.to_owned()),
    }
    for directory in &directories {
        for name in &names {
            push(directory.clone(), name, extension);
        }
        for other in ["java", "kt"] {
            for name in &names {
                push(directory.clone(), name, other);
            }
        }
    }
    candidates
}

/// The `package` of a Java/Kotlin source text.
fn package_of(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let name = line.trim().strip_prefix("package ")?;
        let name = name.trim_end_matches(';').trim();
        (!name.is_empty()).then(|| name.to_owned())
    })
}

fn test_source(path: &Path, package: Option<&str>) -> String {
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Test");
    let kotlin = path.extension().is_some_and(|e| e == "kt");
    let semicolon = if kotlin { "" } else { ";" };
    let header = package
        .map(|package| format!("package {package}{semicolon}\n\n"))
        .unwrap_or_default();
    format!("{header}class {name} {{\n}}\n")
}

impl IdeState {
    fn existing_counterparts(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        let root = self.project.as_ref().ok_or("no project is open")?.root();
        Ok(counterparts(path)
            .into_iter()
            .filter(|candidate| root.join(candidate).is_file())
            .collect())
    }

    fn active_jvm_source(&self) -> Result<PathBuf, String> {
        let document = self
            .documents
            .get(self.active_document.ok_or("no active document")?)
            .ok_or("no active document")?;
        if is_jvm_source(&document.path) {
            Ok(document.path.clone())
        } else {
            Err("Go to Test works in Java and Kotlin files".into())
        }
    }

    fn open_counterparts(&mut self, found: Vec<PathBuf>, what: &str) -> Result<(), String> {
        if let [only] = found.as_slice() {
            return self.navigate_to(&NavigationItem {
                path: only.clone(),
                external: false,
                line: 0,
                column: 0,
                end_column: 0,
                line_text: String::new(),
            });
        }
        self.navigation.choices = Some(NavigationList {
            kind: NavigationKind::Implementation,
            title: format!("Choose {what}"),
            items: found
                .into_iter()
                .map(|path| NavigationItem {
                    line_text: path.display().to_string(),
                    path,
                    external: false,
                    line: 0,
                    column: 0,
                    end_column: 0,
                })
                .collect(),
            truncated: false,
        });
        Ok(())
    }

    /// Go to Test / Test Subject. A source without a test gets one created.
    pub fn go_to_test(&mut self) -> Result<(), String> {
        let path = self.active_jvm_source()?;
        let found = self.existing_counterparts(&path)?;
        if !found.is_empty() {
            let what = if subject_name(path.file_stem().and_then(|s| s.to_str()).unwrap_or(""))
                .is_some()
            {
                "Test Subject"
            } else {
                "Test"
            };
            return self.open_counterparts(found, what);
        }
        self.create_test()
    }

    /// Create Test: writes `<Name>Test` for the active source in the test
    /// source set (opening it when it exists already).
    pub fn create_test(&mut self) -> Result<(), String> {
        let path = self.active_jvm_source()?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("the file has no name")?
            .to_owned();
        if subject_name(&stem).is_some() || locate(&path).is_some_and(|l| is_test_set(&l.set)) {
            return Err(format!(
                "{stem} is a test already; there is nothing to create"
            ));
        }
        let package = self
            .documents
            .iter()
            .find(|document| document.path == path)
            .and_then(|document| package_of(&document.text.text()));
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("java")
            .to_owned();
        let target = counterparts(&path)
            .into_iter()
            .find(|candidate| {
                candidate.file_stem().and_then(|s| s.to_str()) == Some(&created_test_name(&stem))
                    && candidate.extension().is_some_and(|e| *e == *extension)
            })
            .ok_or("cannot decide where to create the test")?;
        let root = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        let absolute = root.join(&target);
        if !absolute.exists() {
            if let Some(parent) = absolute.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("{}: {error}", parent.display()))?;
            }
            std::fs::write(&absolute, test_source(&target, package.as_deref()))
                .map_err(|error| format!("{}: {error}", target.display()))?;
            for ancestor in target.ancestors().skip(1) {
                if !ancestor.as_os_str().is_empty() {
                    self.expanded_dirs.insert(ancestor.to_owned());
                }
            }
            self.refresh_files();
            self.status = format!("Created {}", target.display());
        }
        let status = std::mem::take(&mut self.status);
        self.navigate_to(&NavigationItem {
            path: target,
            external: false,
            line: 0,
            column: 0,
            end_column: 0,
            line_text: String::new(),
        })?;
        if !status.is_empty() {
            self.status = status;
        }
        Ok(())
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let ready = |state: &IdeState| state.active_jvm_source().is_ok();
    registry
        .register(Command::new(
            spec(
                "navigate.test",
                "Go to Test / Test Subject",
                "Navigate",
                Some("⇧⌘T"),
                "{}",
            ),
            ready,
            |state: &mut IdeState, _| {
                state.go_to_test().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("navigate.createTest", "Create Test", "Navigate", None, "{}"),
            ready,
            |state: &mut IdeState, _| {
                state.create_test().map_err(CommandError::Failed)?;
                Ok(json!({"created": true}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IdeState;

    #[test]
    fn subjects_and_tests_pair_across_source_sets() {
        let tests = counterparts(Path::new("app/src/main/java/p/q/Foo.java"));
        assert_eq!(tests[0], Path::new("app/src/test/java/p/q/FooTest.java"));
        assert!(tests.contains(&PathBuf::from("app/src/test/kotlin/p/q/FooTests.kt")));
        let subjects = counterparts(Path::new("app/src/test/java/p/q/FooTest.java"));
        assert_eq!(subjects[0], Path::new("app/src/main/java/p/q/Foo.java"));
        assert!(counterparts(Path::new("src/test/java/p/Helper.java")).is_empty());
        assert_eq!(subject_name("TestFoo").as_deref(), Some("Foo"));
        assert_eq!(subject_name("Testing"), None);
        assert_eq!(subject_name("Test"), None);
    }

    #[test]
    fn kotlin_tests_are_created_beside_the_conventional_location() {
        let tests = counterparts(Path::new("src/main/kotlin/a/Bar.kt"));
        assert_eq!(tests[0], Path::new("src/test/kotlin/a/BarTest.kt"));
        assert_eq!(
            test_source(&tests[0], Some("a")),
            "package a\n\nclass BarTest {\n}\n"
        );
        assert_eq!(package_of("package a.b;\nclass X {}"), Some("a.b".into()));
    }

    #[test]
    fn go_to_test_creates_then_toggles_and_offers_several() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-test-nav-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let main = root.join("src/main/java/p");
        std::fs::create_dir_all(&main).unwrap();
        std::fs::write(main.join("Foo.java"), "package p;\n\npublic class Foo {}\n").unwrap();
        let commands = crate::built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "src/main/java/p/Foo.java"}),
            )
            .unwrap();
        let active =
            |state: &IdeState| state.documents[state.active_document.unwrap()].path.clone();
        // No test yet: Go to Test creates one with the subject's package.
        commands
            .execute("navigate.test", &mut state, json!({}))
            .unwrap();
        let created = PathBuf::from("src/test/java/p/FooTest.java");
        assert_eq!(active(&state), created);
        assert_eq!(
            std::fs::read_to_string(root.join(&created)).unwrap(),
            "package p;\n\nclass FooTest {\n}\n"
        );
        // From the test back to the subject, and again to the test.
        commands
            .execute("navigate.test", &mut state, json!({}))
            .unwrap();
        assert_eq!(active(&state), Path::new("src/main/java/p/Foo.java"));
        commands
            .execute("navigate.test", &mut state, json!({}))
            .unwrap();
        assert_eq!(active(&state), created);
        // Create Test on a test is refused.
        assert!(
            commands
                .execute("navigate.createTest", &mut state, json!({}))
                .is_err()
        );
        // A second test makes Go to Test ask.
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "src/main/java/p/Foo.java"}),
            )
            .unwrap();
        std::fs::write(
            root.join("src/test/java/p/FooTests.java"),
            "class FooTests {}\n",
        )
        .unwrap();
        commands
            .execute("navigate.test", &mut state, json!({}))
            .unwrap();
        let choices = state.navigation.choices.as_ref().expect("chooser");
        assert_eq!(choices.items.len(), 2);
        let _ = std::fs::remove_dir_all(root);
    }
}
