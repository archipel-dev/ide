//! New Java/Kotlin class creation: package inference, name validation, templates, and the
//! `project.newJavaClass` / `project.newKotlinFile` commands.

use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_text::Selection;
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

/// What a new Java file declares (the kind list of the "New Java Class" popup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaKind {
    Class,
    Interface,
    Enum,
    Record,
    Annotation,
}

impl JavaKind {
    pub const ALL: [JavaKind; 5] = [
        JavaKind::Class,
        JavaKind::Interface,
        JavaKind::Enum,
        JavaKind::Record,
        JavaKind::Annotation,
    ];

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    pub fn id(self) -> &'static str {
        match self {
            JavaKind::Class => "class",
            JavaKind::Interface => "interface",
            JavaKind::Enum => "enum",
            JavaKind::Record => "record",
            JavaKind::Annotation => "annotation",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            JavaKind::Class => "Class",
            JavaKind::Interface => "Interface",
            JavaKind::Enum => "Enum",
            JavaKind::Record => "Record",
            JavaKind::Annotation => "Annotation",
        }
    }

    fn declaration(self, name: &str) -> String {
        match self {
            JavaKind::Class => format!("public class {name}"),
            JavaKind::Interface => format!("public interface {name}"),
            JavaKind::Enum => format!("public enum {name}"),
            JavaKind::Record => format!("public record {name}()"),
            JavaKind::Annotation => format!("public @interface {name}"),
        }
    }
}

/// What a new Kotlin file declares (the kind list of the "New Kotlin Class/File" popup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KotlinKind {
    Class,
    Interface,
    Enum,
    Object,
    File,
}

impl KotlinKind {
    pub const ALL: [KotlinKind; 5] = [
        KotlinKind::Class,
        KotlinKind::Interface,
        KotlinKind::Enum,
        KotlinKind::Object,
        KotlinKind::File,
    ];

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    pub fn id(self) -> &'static str {
        match self {
            KotlinKind::Class => "class",
            KotlinKind::Interface => "interface",
            KotlinKind::Enum => "enum",
            KotlinKind::Object => "object",
            KotlinKind::File => "file",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            KotlinKind::Class => "Class",
            KotlinKind::Interface => "Interface",
            KotlinKind::Enum => "Enum class",
            KotlinKind::Object => "Object",
            KotlinKind::File => "File",
        }
    }
}

/// A file ready to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub text: String,
    /// Caret position (in chars) inside the declaration body.
    pub caret: usize,
}

/// Where a new class goes: its project-relative file and package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub file: PathBuf,
    pub package: String,
    pub simple_name: String,
}

const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
    "true",
    "false",
    "null",
    "_",
];

const KOTLIN_KEYWORDS: &[&str] = &[
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
];

fn is_identifier(segment: &str, keywords: &[&str]) -> bool {
    let mut chars = segment.chars();
    let valid_start = |c: char| c.is_alphabetic() || c == '_' || c == '$';
    chars.next().is_some_and(valid_start)
        && segment.chars().all(|c| valid_start(c) || c.is_numeric())
        && !keywords.contains(&segment)
}

/// Package declared by a directory below a source root (`src/main/java`,
/// `src/test/java`, `src/main/kotlin`, `src/<set>/java|kotlin`); empty outside one.
pub fn infer_package(dir: &Path) -> Result<Vec<String>, String> {
    let parts: Vec<String> = dir
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let package = (0..parts.len().saturating_sub(2))
        .find(|&i| parts[i] == "src" && matches!(parts[i + 2].as_str(), "java" | "kotlin"))
        .map(|i| parts[i + 3..].to_vec())
        .unwrap_or_default();
    for segment in &package {
        if !is_identifier(segment, JAVA_KEYWORDS) {
            return Err(format!(
                "'{segment}' in {} is not a valid package name",
                dir.display()
            ));
        }
    }
    Ok(package)
}

/// Resolves `dir` + a possibly dotted `name` (`com.foo.Bar` creates sub-packages) to the
/// file and package of the new type.
pub fn place(
    dir: &Path,
    name: &str,
    extension: &str,
    keywords: &[&str],
) -> Result<Placement, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a name for the new class".into());
    }
    let segments: Vec<&str> = name.split('.').collect();
    for segment in &segments {
        if !is_identifier(segment, keywords) {
            return Err(format!(
                "'{name}' is not a valid class name: '{segment}' is not a valid identifier"
            ));
        }
    }
    let (simple, subpackages) = segments.split_last().expect("split yields one segment");
    let mut package = infer_package(dir)?;
    package.extend(subpackages.iter().map(|s| s.to_string()));
    let mut file = dir.to_owned();
    file.extend(subpackages);
    file.push(format!("{simple}.{extension}"));
    Ok(Placement {
        file,
        package: package.join("."),
        simple_name: simple.to_string(),
    })
}

fn with_body(header: String, package: &str, keyword_package_semicolon: bool) -> Rendered {
    let mut text = String::new();
    if !package.is_empty() {
        text.push_str(&format!(
            "package {package}{}\n\n",
            if keyword_package_semicolon { ";" } else { "" }
        ));
    }
    text.push_str(&header);
    text.push_str(" {\n    ");
    let caret = text.chars().count();
    text.push_str("\n}\n");
    Rendered { text, caret }
}

/// A built-in file template as Settings | Editor | File and Code Templates lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTemplate {
    pub group: &'static str,
    pub name: &'static str,
    pub extension: &'static str,
    /// The template body with `${PACKAGE_NAME}` / `${NAME}` placeholders.
    pub text: String,
}

/// The templates behind New | Java Class and New | Kotlin Class/File.
pub fn file_templates() -> Vec<FileTemplate> {
    let (package, name) = ("${PACKAGE_NAME}", "${NAME}");
    let java = JavaKind::ALL.into_iter().map(|kind| FileTemplate {
        group: "Java",
        name: kind.label(),
        extension: "java",
        text: render_java(kind, package, name).text,
    });
    let kotlin = KotlinKind::ALL.into_iter().map(|kind| FileTemplate {
        group: "Kotlin",
        name: kind.label(),
        extension: "kt",
        text: render_kotlin(kind, package, name).text,
    });
    java.chain(kotlin).collect()
}

pub fn render_java(kind: JavaKind, package: &str, name: &str) -> Rendered {
    with_body(kind.declaration(name), package, true)
}

pub fn render_kotlin(kind: KotlinKind, package: &str, name: &str) -> Rendered {
    let header = match kind {
        KotlinKind::Class => format!("class {name}"),
        KotlinKind::Interface => format!("interface {name}"),
        KotlinKind::Enum => format!("enum class {name}"),
        KotlinKind::Object => format!("object {name}"),
        KotlinKind::File => {
            let text = if package.is_empty() {
                String::new()
            } else {
                format!("package {package}\n\n")
            };
            let caret = text.chars().count();
            return Rendered { text, caret };
        }
    };
    with_body(header, package, false)
}

impl IdeState {
    /// Project-relative form of a directory given relative or absolute.
    fn relative_dir(&self, dir: &str) -> Result<PathBuf, String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let path = Path::new(dir);
        if path.is_absolute() {
            path.strip_prefix(project.root())
                .map(Path::to_owned)
                .map_err(|_| format!("{dir} is outside the project"))
        } else {
            Ok(path.to_owned())
        }
    }

    /// Creates the file from `rendered`, opens it with the caret in the body.
    fn create_from_template(
        &mut self,
        placement: &Placement,
        rendered: &Rendered,
    ) -> Result<PathBuf, String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        if project.root().join(&placement.file).exists() {
            return Err(format!("{} already exists", placement.file.display()));
        }
        project
            .create_file(&placement.file)
            .and_then(|()| project.write_text(&placement.file, &rendered.text))
            .map_err(|error| error.to_string())?;
        for ancestor in placement.file.ancestors().skip(1) {
            if !ancestor.as_os_str().is_empty() {
                self.expanded_dirs.insert(ancestor.to_owned());
            }
        }
        self.refresh_files();
        self.open_file(&placement.file)?;
        if let Some(document) = self.active_document.and_then(|i| self.documents.get_mut(i)) {
            let caret = rendered.caret.min(document.text.len_chars());
            document.selections.set_all(vec![Selection::caret(caret)]);
        }
        self.status = format!("Created {}", placement.file.display());
        Ok(placement.file.clone())
    }

    pub fn new_java_class(&mut self, dir: &str, name: &str, kind: &str) -> Result<Value, String> {
        let kind = JavaKind::parse(kind).ok_or_else(|| {
            format!("unknown kind '{kind}' (class, interface, enum, record, annotation)")
        })?;
        let dir = self.relative_dir(dir)?;
        let placement = place(&dir, name, "java", JAVA_KEYWORDS)?;
        let rendered = render_java(kind, &placement.package, &placement.simple_name);
        let path = self.create_from_template(&placement, &rendered)?;
        Ok(json!({"path": path, "package": placement.package}))
    }

    pub fn new_kotlin_file(&mut self, dir: &str, name: &str, kind: &str) -> Result<Value, String> {
        let kind = KotlinKind::parse(kind).ok_or_else(|| {
            format!("unknown kind '{kind}' (class, interface, enum, object, file)")
        })?;
        let dir = self.relative_dir(dir)?;
        let placement = place(&dir, name, "kt", KOTLIN_KEYWORDS)?;
        let rendered = render_kotlin(kind, &placement.package, &placement.simple_name);
        let path = self.create_from_template(&placement, &rendered)?;
        Ok(json!({"path": path, "package": placement.package}))
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "dialog.injectPath",
                "Answer Next Native Dialog",
                "Project",
                None,
                r#"{"path":"string"}"#,
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                state.injected_dialog_paths.push_back(path.clone());
                Ok(json!({"queued": path}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.newJavaClass",
                "New Java Class",
                "Project",
                None,
                r#"{"dir":"string","name":"string","kind":"class|interface|enum|record|annotation"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                state
                    .new_java_class(
                        &string_param(&params, "dir")?,
                        &string_param(&params, "name")?,
                        &string_param(&params, "kind")?,
                    )
                    .map_err(CommandError::Failed)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.newKotlinFile",
                "New Kotlin Class/File",
                "Project",
                None,
                r#"{"dir":"string","name":"string","kind":"class|interface|enum|object|file"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                state
                    .new_kotlin_file(
                        &string_param(&params, "dir")?,
                        &string_param(&params, "name")?,
                        &string_param(&params, "kind")?,
                    )
                    .map_err(CommandError::Failed)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn project_state(name: &str) -> (IdeState, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-newclass-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src/main/java/com/example")).unwrap();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        (state, root.canonicalize().unwrap())
    }

    fn run(state: &mut IdeState, command: &str, params: Value) -> Result<Value, CommandError> {
        let mut registry = CommandRegistry::default();
        register(&mut registry);
        registry.execute(command, state, params)
    }

    #[test]
    fn command_creates_files_and_opens_with_caret_in_body() {
        let (mut state, root) = project_state("create");
        let result = run(
            &mut state,
            "project.newJavaClass",
            json!({"dir": "src/main/java/com/example", "name": "util.Foo", "kind": "record"}),
        )
        .unwrap();
        assert_eq!(result["package"], "com.example.util");
        let file = root.join("src/main/java/com/example/util/Foo.java");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "package com.example.util;\n\npublic record Foo() {\n    \n}\n"
        );
        let document = &state.documents[state.active_document.unwrap()];
        assert!(document.path.ends_with("Foo.java"));
        let caret = document.selections.primary().head;
        let text = document
            .text
            .slice(caret, document.text.len_chars())
            .unwrap();
        assert_eq!(text, "\n}\n");
        // Absolute directories inside the project work too; Kotlin gets .kt.
        let dir = root.join("src/main/java/com/example");
        run(
            &mut state,
            "project.newKotlinFile",
            json!({"dir": dir, "name": "Util", "kind": "file"}),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("Util.kt")).unwrap(),
            "package com.example\n\n"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn command_rejects_bad_names_existing_files_and_outside_dirs() {
        let (mut state, root) = project_state("reject");
        let params =
            |name: &str| json!({"dir": "src/main/java/com/example", "name": name, "kind": "class"});
        let bad = run(&mut state, "project.newJavaClass", params("1Bad")).unwrap_err();
        assert!(bad.to_string().contains("not a valid"), "{bad}");
        run(&mut state, "project.newJavaClass", params("Foo")).unwrap();
        let again = run(&mut state, "project.newJavaClass", params("Foo")).unwrap_err();
        assert!(again.to_string().contains("already exists"), "{again}");
        let kind = run(
            &mut state,
            "project.newJavaClass",
            json!({"dir": "src", "name": "A", "kind": "nope"}),
        )
        .unwrap_err();
        assert!(kind.to_string().contains("unknown kind"), "{kind}");
        let escape = run(
            &mut state,
            "project.newJavaClass",
            json!({"dir": "/etc", "name": "A", "kind": "class"}),
        )
        .unwrap_err();
        assert!(
            escape.to_string().contains("outside the project"),
            "{escape}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn infers_packages_below_source_roots() {
        for (dir, expected) in [
            ("src/main/java/com/example", vec!["com", "example"]),
            ("app/src/test/java/org/x/y", vec!["org", "x", "y"]),
            ("src/main/kotlin/dev", vec!["dev"]),
            ("src/main/java", vec![]),
            ("docs/notes", vec![]),
        ] {
            assert_eq!(infer_package(Path::new(dir)).unwrap(), expected, "{dir}");
        }
        assert!(infer_package(Path::new("src/main/java/my-pkg")).is_err());
        assert!(infer_package(Path::new("src/main/java/com/class")).is_err());
    }

    #[test]
    fn dotted_names_create_subpackages() {
        let placed = place(
            Path::new("src/main/java/com/example"),
            "app.model.User",
            "java",
            JAVA_KEYWORDS,
        )
        .unwrap();
        assert_eq!(placed.package, "com.example.app.model");
        assert_eq!(placed.simple_name, "User");
        assert_eq!(
            placed.file,
            Path::new("src/main/java/com/example/app/model/User.java")
        );
    }

    #[test]
    fn invalid_names_are_rejected() {
        for name in [
            "", "  ", "1Foo", "Foo.", ".Foo", "a..B", "my class", "class", "a/b", "Foo-Bar",
        ] {
            assert!(
                place(Path::new("src/main/java"), name, "java", JAVA_KEYWORDS).is_err(),
                "{name:?}"
            );
        }
    }

    #[test]
    fn renders_java_templates() {
        let class = render_java(JavaKind::Class, "com.example", "Foo");
        assert_eq!(
            class.text,
            "package com.example;\n\npublic class Foo {\n    \n}\n"
        );
        assert_eq!(&class.text[class.caret..], "\n}\n");
        let record = render_java(JavaKind::Record, "", "P");
        assert_eq!(record.text, "public record P() {\n    \n}\n");
        assert!(
            render_java(JavaKind::Annotation, "a", "A")
                .text
                .contains("public @interface A {")
        );
        assert!(
            render_java(JavaKind::Enum, "a", "E")
                .text
                .contains("public enum E {")
        );
        assert!(
            render_java(JavaKind::Interface, "a", "I")
                .text
                .contains("public interface I {")
        );
    }

    #[test]
    fn renders_kotlin_templates() {
        let class = render_kotlin(KotlinKind::Class, "com.example", "Foo");
        assert_eq!(class.text, "package com.example\n\nclass Foo {\n    \n}\n");
        let file = render_kotlin(KotlinKind::File, "com.example", "Util");
        assert_eq!(file.text, "package com.example\n\n");
        assert_eq!(file.caret, file.text.chars().count());
    }
}
