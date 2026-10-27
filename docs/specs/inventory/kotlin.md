# Kotlin

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (45)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `CheckComponentsUsageSearchAction` | Check Component Functions Usage Search |  |  | MISSING | `plugins/kotlin` |
| `ConfigureKotlinInProject` | Configure Kotlin in Project | `KotlinToolsGroup` |  | MISSING | `plugins/kotlin` |
| `ConvertJavaToKotlin` | Convert Java to Kotlin | `CodeMenu`, `ProjectViewPopupMenu`, `EditorTabPopupMenu` | `control alt shift K` | MISSING | `plugins/kotlin` |
| `ConvertJavaToKotlinInGroup` | Static Analysis | `ConvertJavaToKotlinGroup` | `control alt shift K` | MISSING | `plugins/kotlin` |
| `CreateIncrementalCompilationBackup` | (set in code) | `KotlinInternalGroup` |  | MISSING | `plugins/kotlin` |
| `DecompileKotlinToJava` | Decompile to Java | `KotlinToolsGroup` |  | MISSING | `plugins/kotlin` |
| `ExtractFunction` | Extract Function… | `IntroduceActionsGroup`, `Floating.CodeToolbar.Extract` | as `ExtractMethod` | MISSING | `plugins/kotlin` |
| `ExtractFunctionToScope` | Extract Function to Scope… | `IntroduceActionsGroup` | `control alt shift M` | MISSING | `plugins/kotlin` |
| `Gradle.MigrateToDaemonJvmCriteria` | Migrate Project to Daemon JVM Criteria |  |  | MISSING | `plugins/kotlin` |
| `InspectBreakpointApplicability` | Inspect Breakpoint Applicability | `KotlinInternalGroup` |  | MISSING | `plugins/kotlin` |
| `IntroduceProperty` | Introduce Property… | `IntroduceActionsGroup` | as `IntroduceField` | MISSING | `plugins/kotlin` |
| `IntroduceTypeAlias` | Introduce Type Alias… | `IntroduceActionsGroup` | `control alt shift A` | MISSING | `plugins/kotlin` |
| `InvalidateK2CachesInternalAction` | Invalidate Kotlin Plugin K2 Mode Caches | `KotlinInternalGroup` |  | MISSING | `plugins/kotlin` |
| `Kotlin.ClearScratch` | Clear Kotlin Scratch |  |  | MISSING | `plugins/kotlin` |
| `Kotlin.Gradle.ShowDslLogs` | Show Kotlin Gradle DSL Logs |  |  | MISSING | `plugins/kotlin` |
| `Kotlin.NewFile` | Kotlin Class/File | `NewGroup` |  | MISSING | `plugins/kotlin` |
| `Kotlin.NewScript` | Kotlin Script | `NewGroup` |  | MISSING | `plugins/kotlin` |
| `Kotlin.RunScratch` | Run Kotlin Scratch | `RunContextPopupGroup` | `control alt W` | MISSING | `plugins/kotlin` |
| `Kotlin.StopScratch` | Stop Scratch Execution |  |  | MISSING | `plugins/kotlin` |
| `Kotlin.XDebugger.ToggleKotlinVariableView` | Show Kotlin Variables Only | `XDebugger.Settings` |  | MISSING | `plugins/kotlin` |
| `KotlinCodeMigration` | Run Code Migrations |  |  | MISSING | `plugins/kotlin` |
| `KotlinCodeMigrationToggle` | Enable Migrations Detection | `KotlinToolsGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinCompilerReferenceIndexVerifierAction` | Show Compiler Index Status | `Internal` |  | MISSING | `plugins/kotlin` |
| `KotlinCriDiagnostics` | Kotlin CRI Diagnostics | `HelpDiagnosticTools` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateDataMethod` | Parameters Function | `KotlinGenerateGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateEqualsAndHashCode` | equals() and hashCode() | `KotlinGenerateGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateMavenCompileExecutionAction` | Kotlin compile execution | `KotlinMavenGenerate` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateMavenPluginAction` | Kotlin plugin | `KotlinMavenGenerate` |  | codeinsight/03-inspections.md CI-ALT-050 | `plugins/kotlin` |
| `KotlinGenerateMavenTestCompileExecutionAction` | Kotlin test-compile execution | `KotlinMavenGenerate` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateSecondaryConstructor` | Secondary Constructor | `KotlinGenerateGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateSetUpMethod` | SetUp Function | `KotlinGenerateGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateTearDownMethod` | TearDown Function | `KotlinGenerateGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateTestMethod` | Test Function | `KotlinGenerateGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinGenerateToString` | toString() | `KotlinGenerateGroup` |  | MISSING | `plugins/kotlin` |
| `KotlinScripts.DefinitionLabel` | Script Definition | `KotlinScripts.EditorToolbar` |  | MISSING | `plugins/kotlin` |
| `KotlinScripts.OpenScriptingSettings` | Kotlin Scripting Settings… |  |  | MISSING | `plugins/kotlin` |
| `KotlinScripts.ShowScriptStructure` | Show Script Structure | `KotlinScripts.EditorToolbar` |  | MISSING | `plugins/kotlin` |
| `KtAddToExcludeListAction` | Do not show hints for current method | `InlayMenu` |  | MISSING | `plugins/kotlin` |
| `LibraryToSourceDependencySupportToggleAction` | Toggle Library to Source Dependency Support |  |  | MISSING | `plugins/kotlin` |
| `ReactivePostOpenProjectActionsAction` | Kotlin Project Post-Open Activity | `KotlinInternalGroup` |  | MISSING | `plugins/kotlin` |
| `ReloadScriptConfiguration` | Reload script configuration | `KotlinScripts.EditorToolbar` | `ctrl alt shift R` | MISSING | `plugins/kotlin` |
| `ShowKotlinBytecode` | Show Kotlin Bytecode | `KotlinToolsGroup` |  | MISSING | `plugins/kotlin` |
| `org.jetbrains.kotlin.onboarding.ShowKotlinDeveloperSurveyNotificationAction` | Show Kotlin Developer Survey Notification | `OnboardingKotlin` |  | MISSING | `plugins/kotlin` |
| `org.jetbrains.kotlin.onboarding.gradle.ShowBuildProcessSatisfactionDialogAction` | Show Kotlin Gradle Build Process Satisfaction Survey | `BuildProcessSatisfaction` |  | MISSING | `plugins/kotlin` |
| `org.jetbrains.kotlin.onboarding.maven.ShowMavenBuildProcessSatisfactionDialogAction` | Show Kotlin Maven Build Process Satisfaction Survey | `MavenBuildProcessSatisfaction` |  | MISSING | `plugins/kotlin` |

## Settings pages (3)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `preferences.language.Kotlin` | Kotlin | language |  | application | MISSING |
| `project.kotlinCompiler` | Kotlin Compiler |  | project.propCompiler | project | MISSING |
| `preferences.language.Kotlin.scripting` | Kotlin Scripting | language | preferences.language.Kotlin | project | MISSING |

## Run configuration types (2)

| Class | Type id | Display name | Plugin | Spec |
|---|---|---|---|---|
| `KotlinStandaloneScriptRunConfigurationType` | `(see class)` | (see class) | plugins/kotlin | MISSING |
| `KotlinRunConfigurationType` | `(see class)` | (see class) | plugins/kotlin | MISSING |
