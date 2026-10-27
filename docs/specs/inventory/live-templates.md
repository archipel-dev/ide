# Live templates and file templates

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (8)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `InsertLiveTemplate` | Insert Live Template… | `CodeMenu` | `control J` | MISSING | `platform/lang-impl` |
| `NextTemplateVariable` | Next Template Variable or Finish In-Place Refactoring | `LookupActions` | `TAB`, `ENTER` | MISSING | `platform/lang-impl` |
| `PrevTemplateParameter` | Previous Live Template Parameter | `TemplateParametersNavigation` |  | MISSING | `platform/lang-api` |
| `PreviousTemplateVariable` | Previous Template Variable | `LookupActions` | `shift TAB` | MISSING | `platform/lang-impl` |
| `SaveAsTemplate` | Save as Live Template… | `CodeMenu` |  | MISSING | `platform/lang-impl` |
| `SelectVirtualTemplateElement` | Show Options Popup for In-place Refactoring | `LookupActions` | `Alt Shift O` | MISSING | `platform/lang-impl` |
| `SurroundWithLiveTemplate` | Surround with Live Template… |  | `control alt J` | MISSING | `platform/lang-impl` |
| `TemplateProjectProperties` | (set in code) | `FileOtherSettingsGroup` |  | MISSING | `platform/platform-impl` |

## Settings pages (3)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `editing.templates` | Live Templates | editor |  | application | MISSING |
| `Template Data Languages` | Template Data Languages | language |  | project | MISSING |
| `fileTemplates` | File and Code Templates | editor |  | project | MISSING |

## Default live templates (102)

Group = template set; contexts = first enabled context options.

| Abbreviation | Group | Description | Contexts | Spec |
|---|---|---|---|---|
| `fori` | Java | Create iteration loop | JAVA_STATEMENT | MISSING |
| `else-if` | Java | Add else-if branch | JAVA_ELSE_PLACE | MISSING |
| `itar` | Java |  | JAVA_STATEMENT | MISSING |
| `itco` | Java |  | JAVA_STATEMENT | MISSING |
| `iten` | Java |  | JAVA_STATEMENT | MISSING |
| `itit` | Java |  | JAVA_STATEMENT | MISSING |
| `itli` | Java |  | JAVA_STATEMENT | MISSING |
| `itm` | Java |  | JAVA_STATEMENT | MISSING |
| `ittok` | Java |  | JAVA_STATEMENT | MISSING |
| `ritar` | Java |  | JAVA_STATEMENT | MISSING |
| `iter` | Java |  | JAVA_STATEMENT | MISSING |
| `inst` | Java |  | JAVA_STATEMENT | MISSING |
| `lst` | Java |  | JAVA_EXPRESSION | MISSING |
| `mn` | Java |  | JAVA_STATEMENT | MISSING |
| `mx` | Java |  | JAVA_STATEMENT | MISSING |
| `psvm` | Java |  | JAVA_NORMAL_CLASS_DECLARATION_WITHOUT_INSTANCE_MAIN | MISSING |
| `main` | Java |  | JAVA_NORMAL_CLASS_DECLARATION_WITHOUT_INSTANCE_MAIN | MISSING |
| `toar` | Java |  | JAVA_EXPRESSION | MISSING |
| `lazy` | Java |  | JAVA_STATEMENT | MISSING |
| `inn` | Java |  | JAVA_STATEMENT | MISSING |
| `ifn` | Java |  | JAVA_STATEMENT | MISSING |
| `geti` | Java |  | JAVA_DECLARATION | MISSING |
| `St` | Java |  | JAVA_STATEMENT, JAVA_DECLARATION | MISSING |
| `psf` | Java |  | JAVA_DECLARATION | MISSING |
| `prsf` | Java |  | JAVA_DECLARATION | MISSING |
| `psfi` | Java |  | JAVA_DECLARATION | MISSING |
| `psfs` | Java |  | JAVA_DECLARATION | MISSING |
| `thr` | Java |  | JAVA_STATEMENT | MISSING |
| `serr` | Java |  | JAVA_STATEMENT | MISSING |
| `serrc` | Java | System.err::println | JAVA_CONSUMER | MISSING |
| `sout` | Java |  | JAVA_STATEMENT | MISSING |
| `iop` | Java |  | JAVA_LANG_IO_STATEMENT | MISSING |
| `ior` | Java |  | JAVA_LANG_IO_STATEMENT | MISSING |
| `soutc` | Java | System.out::println | JAVA_CONSUMER | MISSING |
| `souf` | Java |  | JAVA_STATEMENT | MISSING |
| `soutm` | Java |  | JAVA_STATEMENT | MISSING |
| `soutp` | Java |  | JAVA_STATEMENT | MISSING |
| `soutv` | Java |  | JAVA_STATEMENT | MISSING |
| `C` | Java |  | JAVA_STATEMENT | MISSING |
| `RL` | Java |  | JAVA_STATEMENT | MISSING |
| `WL` | Java |  | JAVA_STATEMENT | MISSING |
| `I` | Java |  | JAVA_STATEMENT | MISSING |
| `sts` | Java |  | JAVA_STATEMENT_WITH_STRUCTURED_CONCURRENCY_STATIC_FACTORY_METHODS_WITH_EXECUTION_EXCEPTION | MISSING |
| `psvm` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `main` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `psvma` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `maina` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `psvm` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |
| `main` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |
| `psvma` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |
| `maina` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |
| `fori` | Java | Create iteration loop | JAVA_STATEMENT | MISSING |
| `else-if` | Java | Add else-if branch | JAVA_ELSE_PLACE | MISSING |
| `itar` | Java |  | JAVA_STATEMENT | MISSING |
| `itco` | Java |  | JAVA_STATEMENT | MISSING |
| `iten` | Java |  | JAVA_STATEMENT | MISSING |
| `itit` | Java |  | JAVA_STATEMENT | MISSING |
| `itli` | Java |  | JAVA_STATEMENT | MISSING |
| `itm` | Java |  | JAVA_STATEMENT | MISSING |
| `ittok` | Java |  | JAVA_STATEMENT | MISSING |
| `ritar` | Java |  | JAVA_STATEMENT | MISSING |
| `iter` | Java |  | JAVA_STATEMENT | MISSING |
| `inst` | Java |  | JAVA_STATEMENT | MISSING |
| `lst` | Java |  | JAVA_EXPRESSION | MISSING |
| `mn` | Java |  | JAVA_STATEMENT | MISSING |
| `mx` | Java |  | JAVA_STATEMENT | MISSING |
| `psvm` | Java |  | JAVA_NORMAL_CLASS_DECLARATION_WITHOUT_INSTANCE_MAIN | MISSING |
| `main` | Java |  | JAVA_NORMAL_CLASS_DECLARATION_WITHOUT_INSTANCE_MAIN | MISSING |
| `toar` | Java |  | JAVA_EXPRESSION | MISSING |
| `lazy` | Java |  | JAVA_STATEMENT | MISSING |
| `inn` | Java |  | JAVA_STATEMENT | MISSING |
| `ifn` | Java |  | JAVA_STATEMENT | MISSING |
| `geti` | Java |  | JAVA_DECLARATION | MISSING |
| `St` | Java |  | JAVA_STATEMENT, JAVA_DECLARATION | MISSING |
| `psf` | Java |  | JAVA_DECLARATION | MISSING |
| `prsf` | Java |  | JAVA_DECLARATION | MISSING |
| `psfi` | Java |  | JAVA_DECLARATION | MISSING |
| `psfs` | Java |  | JAVA_DECLARATION | MISSING |
| `thr` | Java |  | JAVA_STATEMENT | MISSING |
| `serr` | Java |  | JAVA_STATEMENT | MISSING |
| `serrc` | Java | System.err::println | JAVA_CONSUMER | MISSING |
| `sout` | Java |  | JAVA_STATEMENT | MISSING |
| `iop` | Java |  | JAVA_LANG_IO_STATEMENT | MISSING |
| `ior` | Java |  | JAVA_LANG_IO_STATEMENT | MISSING |
| `soutc` | Java | System.out::println | JAVA_CONSUMER | MISSING |
| `souf` | Java |  | JAVA_STATEMENT | MISSING |
| `soutm` | Java |  | JAVA_STATEMENT | MISSING |
| `soutp` | Java |  | JAVA_STATEMENT | MISSING |
| `soutv` | Java |  | JAVA_STATEMENT | MISSING |
| `C` | Java |  | JAVA_STATEMENT | MISSING |
| `RL` | Java |  | JAVA_STATEMENT | MISSING |
| `WL` | Java |  | JAVA_STATEMENT | MISSING |
| `I` | Java |  | JAVA_STATEMENT | MISSING |
| `sts` | Java |  | JAVA_STATEMENT_WITH_STRUCTURED_CONCURRENCY_STATIC_FACTORY_METHODS_WITH_EXECUTION_EXCEPTION | MISSING |
| `psvm` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `main` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `psvma` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `maina` | Java//Instance 'main' methods for implicitly declared classes |  | JAVA_IMPLICIT_CLASS_DECLARATION | MISSING |
| `psvm` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |
| `main` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |
| `psvma` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |
| `maina` | Java//Instance 'main' methods for normal classes |  | JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN | MISSING |

## Live template contexts (49)

| Context id | Base context | Plugin | Spec |
|---|---|---|---|
| `JAVA_CODE` |  | java/java-impl | MISSING |
| `JAVA_STATEMENT` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_CONSUMER` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_EXPRESSION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_ELSE_PLACE` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_DECLARATION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_IMPLICIT_CLASS_DECLARATION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_NORMAL_CLASS_DECLARATION_WITHOUT_INSTANCE_MAIN` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_LANG_IO_STATEMENT` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_STATEMENT_WITH_STRUCTURED_CONCURRENCY_STATIC_FACTORY_METHODS_WITH_EXECUTION_EXCEPTION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_COMMENT` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_STRING` | `JAVA_CODE` | java/java-impl | MISSING |
| `COMPLETION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_CODE` |  | java/java-impl | MISSING |
| `JAVA_STATEMENT` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_CONSUMER` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_EXPRESSION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_ELSE_PLACE` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_DECLARATION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_IMPLICIT_CLASS_DECLARATION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_NORMAL_CLASS_DECLARATION_WITHOUT_INSTANCE_MAIN` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_LANG_IO_STATEMENT` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_NORMAL_CLASS_DECLARATION_WITH_INSTANCE_MAIN` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_STATEMENT_WITH_STRUCTURED_CONCURRENCY_STATIC_FACTORY_METHODS_WITH_EXECUTION_EXCEPTION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_COMMENT` | `JAVA_CODE` | java/java-impl | MISSING |
| `JAVA_STRING` | `JAVA_CODE` | java/java-impl | MISSING |
| `COMPLETION` | `JAVA_CODE` | java/java-impl | MISSING |
| `JSON` |  | json/backend | MISSING |
| `JSON_STRING_VALUES` | `JSON` | json/backend | MISSING |
| `JSON_PROPERTY_KEYS` | `JSON` | json/backend | MISSING |
| `OTHER` |  | platform/analysis-api | MISSING |
| `KOTLIN` |  | plugins/kotlin | MISSING |
| `KOTLIN_TOPLEVEL` | `KOTLIN` | plugins/kotlin | MISSING |
| `KOTLIN_STATEMENT` | `KOTLIN` | plugins/kotlin | MISSING |
| `KOTLIN_CLASS` | `KOTLIN` | plugins/kotlin | MISSING |
| `KOTLIN_EXPRESSION` | `KOTLIN` | plugins/kotlin | MISSING |
| `KOTLIN_COMMENT` | `KOTLIN` | plugins/kotlin | MISSING |
| `KOTLIN_OBJECT_DECLARATION` | `KOTLIN` | plugins/kotlin | MISSING |
| `MARKDOWN` |  | plugins/markdown | MISSING |
| `MAVEN` |  | plugins/maven | MISSING |
| `Properties` |  | plugins/properties | MISSING |
| `SHELL_SCRIPT` |  | plugins/sh | MISSING |
| `HTML_TEXT` | `HTML` | xml/xml-ui-common | MISSING |
| `HTML` |  | xml/xml-ui-common | MISSING |
| `XSL_TEXT` | `XML` | xml/xml-ui-common | MISSING |
| `XML` |  | xml/xml-ui-common | MISSING |
| `XML_TEXT` | `XML` | xml/xml-ui-common | MISSING |
| `XML_TAG` | `XML` | xml/xml-ui-common | MISSING |
