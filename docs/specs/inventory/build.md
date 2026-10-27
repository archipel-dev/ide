# Build, compile and artifacts

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (19)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `ActivateAntToolWindow` | Ant | `ActivateToolWindowActions` |  | MISSING | `plugins/ant` |
| `AddAntBuildFile` | Add as Ant Build File | `ToolsXmlGroup`, `EditorPopupMenu`, `ProjectViewPopupMenu` (+2) |  | MISSING | `plugins/ant` |
| `BuildArtifact` | Build Artifacts… | `Java.BuildMenu` |  | MISSING | `java/java-backend` |
| `BuildViewFilterSuccessful` | Show Successful Steps | `BuildViewFiltering` |  | MISSING | `platform/buildView` |
| `BuildViewFilterWarnings` | Show Warnings | `BuildViewFiltering` |  | MISSING | `platform/buildView` |
| `CompilationChartsScrollToEndAction` | Scroll to End In Compilation Charts |  |  | MISSING | `plugins/compilation-charts` |
| `CompilationChartsZoomInAction` | Zoom In Compilation Charts |  | as `ExpandAll` | MISSING | `plugins/compilation-charts` |
| `CompilationChartsZoomOutAction` | Zoom Out Compilation Charts |  | as `CollapseAll` | MISSING | `plugins/compilation-charts` |
| `CompilationChartsZoomResetAction` | Reset Zoom In Compilation Charts |  | `shift ctrl EQUALS` | MISSING | `plugins/compilation-charts` |
| `Compile` | Rebuild | `Java.BuildMenu`, `EditorTabCompileGroup`, `ProjectViewCompileGroup` (+2) | `control shift F9` | java/02-resolve.md JAVA-RES-028 | `java/java-backend` |
| `CompileDirty` | Build Project | `Java.BuildMenu`, `TouchBarDefault`, `RightToolbarSideGroup` (+1) | `control F9` | MISSING | `java/java-backend` |
| `CompileFile` | Recompile | `Java.BuildMenu` | as `Compile` | MISSING | `java/java-backend` |
| `CompileProject` | Rebuild Project | `Java.BuildMenu` |  | MISSING | `java/java-backend` |
| `ExcludeFromValidation` | (set in code) | `CompilerErrorViewPopupMenu` |  | MISSING | `java/java-backend` |
| `GenerateJavadoc` | Generate Javadoc… | `ToolsBasicGroup` |  | MISSING | `java/java-backend` |
| `MakeModule` | Build Module | `Java.BuildMenu`, `ProjectViewCompileGroup` |  | MISSING | `java/java-backend` |
| `PackageFile` | (set in code) | `EditorPopupMenu`, `ProjectViewPopupMenu`, `ScopeViewPopupMenu` | as `Compile` | MISSING | `java/java-backend` |
| `PinBuildView` | (set in code) | `BuildViewToolbar` |  | MISSING | `platform/buildView` |
| `RunTargetAction` | Run Ant Target | `BuildMenu` | `control shift F10` | MISSING | `plugins/ant` |

## Settings pages (5)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `promo.kubernetes` | Kubernetes | build |  | application | MISSING |
| `promo.profiler` | Java Profiler | build |  | application | MISSING |
| `RemoteServers` | Clouds | build |  | application | MISSING |
| `project.propCompiler` | Compiler | build |  | project | MISSING |
| `com.intellij.jarRepository.settings.RemoteRepositoriesConfigurable` | Remote Jar Repositories | build |  | project | MISSING |

## Tool windows (1)

| Id | Title | Anchor | Flags | Factory | Spec |
|---|---|---|---|---|---|
| `Ant` | Ant | right |  | AntToolWindowFactory | MISSING |

## Run configuration types (1)

| Class | Type id | Display name | Plugin | Spec |
|---|---|---|---|---|
| `AntRunConfigurationType` | `AntRunConfiguration` | Ant Target | plugins/ant | MISSING |
