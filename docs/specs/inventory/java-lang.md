# Java language support (generate, code, compiler specific)

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Actions (92)

| Action id | Text | Placement (parent groups) | Default shortcut | Spec | Source |
|---|---|---|---|---|---|
| `Actions.ActionsPlugin.GenerateToString` | toString() | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `AddFrameworkSupport` | Add Framework Support… | `ProjectViewPopupMenuSettingsGroup` |  | MISSING | `java/java-backend` |
| `AddJdkAction` | Add JDK from Disk… |  |  | MISSING | `java/java-backend` |
| `CompactSourceFile` | (set in code) | `NewGroup1` |  | MISSING | `java/java-backend` |
| `ConvertModuleGroupsToQualifiedNames` | (set in code) | `FileMainSettingsGroup` |  | MISSING | `java/java-backend` |
| `CreateLibraryFromFile` | Add as Library… | `Java.ImportFrom.Group` |  | MISSING | `java/java-backend` |
| `DelegateMethods` | Delegate Methods… | `CodeMenu`, `GenerateGroup` |  | MISSING | `platform/lang-impl` |
| `DisableRepositoryLibrariesSha256Checksum` | Disable SHA256 Checksum for All Libraries | `JarRepositoryLibraries` |  | MISSING | `java/java-backend` |
| `DownloadJdkAction` | Download JDK… |  |  | MISSING | `java/java-backend` |
| `EnableAllRepositoryLibrariesSha256Checksum` | Enable SHA256 Checksum for All Libraries | `JarRepositoryLibraries` |  | MISSING | `java/java-backend` |
| `ExpressionStatistics` | Collect Java Expression Statistics | `Internal.Java` |  | MISSING | `java/java-backend` |
| `Generate` | Generate… | `CodeMenu`, `EditorLangPopupMenu` | `alt INSERT` | java/07-inspections.md JAVA-INS-031 | `platform/lang-impl` |
| `GenerateAfterClassMethod` | AfterClass Method | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateBeforeClassMethod` | BeforeClass Method | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateConstructor` | Constructor | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateCreateUI` | createUI(…) | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateDataMethod` | Parameters Method | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateEquals` | equals() and hashCode() | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateGetter` | Getter | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateGetterAndSetter` | Getter and Setter | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateLogField` | Logger | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateModuleDescriptors` | Generate module-info Descriptors | `CodeMenu` |  | MISSING | `java/java-backend` |
| `GenerateProviderMethod` | provider() | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateSetUpMethod` | SetUp Method | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateSetter` | Setter | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateSuperMethodCall` | Super Method Call | `JavaGenerateGroup2` |  | MISSING | `java/java-backend` |
| `GenerateTearDownMethod` | TearDown Method | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateTestDataPathFiles` | @TestDataPath Files | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateTestDataPathMethods` | @TestDataPath Methods | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateTestMethod` | Test Method | `JavaGenerateGroup1` |  | MISSING | `java/java-backend` |
| `GenerateVisitorByHierarchy` | Generate Hierarchy Visitor… | `Internal.Java` |  | MISSING | `java/java-backend` |
| `GuessRemoteRepositoryForEachRepositoryLibrary` | Guess and Bind Remote Repositories for All Libraries | `JarRepositoryLibraries` |  | MISSING | `java/java-backend` |
| `ImplementMethods` | Implement Methods… | `GenerateGroup`, `CodeMenu` | `control I` | MISSING | `platform/lang-impl` |
| `InferNullity` | Infer Nullity… | `AnalyzeActions` |  | MISSING | `java/java-backend` |
| `InsertAnnotationAction` | Insert Annotation(s) | `InlayMenu` |  | MISSING | `java/java-backend` |
| `JShell.Console` | JShell Console… | `ToolsMenu` |  | MISSING | `plugins/jshell` |
| `LibraryUsageStatistics` | Show Library Usage Information of Import |  |  | MISSING | `java/java-impl` |
| `LoadUnloadModules` | Load/Unload Modules… | `ProjectViewPopupMenuSettingsGroup` |  | MISSING | `java/java-backend` |
| `ManageProjectTemplates` | Manage Project Templates… | `Java.ProjectTemplate.Group` |  | MISSING | `java/java-backend` |
| `MethodDuplicates` | Find and Replace Code Duplicates… | `RefactoringMenu2` |  | MISSING | `java/java-backend` |
| `MethodOverloadSwitchDown` | Next Method Overload |  | `control DOWN` | MISSING | `java/java-backend` |
| `MethodOverloadSwitchUp` | Previous Method Overload |  | `control UP` | MISSING | `java/java-backend` |
| `NewClass` | Java Class | `NewGroup1` |  | MISSING | `java/java-backend` |
| `NewMigration` | Create New Migration… | `MigrationMenu` |  | MISSING | `java/java-backend` |
| `NewPackageInfo` | (set in code) | `NewJavaSpecialFile` |  | MISSING | `java/java-backend` |
| `OpenModuleSettings` | Module Settings | `ProjectViewPopupMenuSettingsGroup` | as `EditSource` | MISSING | `java/java-backend` |
| `OverrideFileTypeAction` | Override File Type | `MarkFileAs` |  | MISSING | `platform/lang-impl` |
| `OverrideMethods` | Override Methods… | `GenerateGroup`, `CodeMenu` | `control O` | MISSING | `platform/lang-impl` |
| `ResolveAllRepositoryLibraries` | Resolve All Maven Libraries | `JarRepositoryLibraries` |  | MISSING | `java/java-backend` |
| `SaveProjectAsTemplate` | Save Project as Template… | `Java.ProjectTemplate.Group` |  | MISSING | `java/java-backend` |
| `ShowBackwardPackageDeps` | Analyze Backward Dependencies… | `ShowPackageDepsGroup` |  | MISSING | `java/java-backend` |
| `ShowDependenciesOnTarget` | Analyze Dependencies on Specified Target | `ShowPackageDepsGroup` |  | MISSING | `java/java-backend` |
| `ShowDiscoveredTests` | Show Affected Tests | `RunTestGroup`, `ChangesViewPopupMenu` |  | MISSING | `java/vcs` |
| `ShowModulesDependencies` | Analyze Module Dependencies… | `ShowPackageDepsGroup` |  | MISSING | `java/java-backend` |
| `ShowPackageCycles` | Analyze Cyclic Dependencies… | `ShowPackageDepsGroup` |  | MISSING | `java/java-backend` |
| `ShowSiblings` | Show Siblings | `QuickActions` |  | MISSING | `java/java-backend` |
| `StaticIconFields` | List Static Icon Fields Initializers | `Internal.Java` |  | MISSING | `java/java-backend` |
| `TestCompilerReferenceServiceFindUsages` | (set in code) |  |  | MISSING | `java/java-backend` |
| `TestCompilerReferenceServiceFunctionalExpressionSearch` | (set in code) |  |  | MISSING | `java/java-backend` |
| `TestCompilerReferenceServiceInheritance` | (set in code) |  |  | MISSING | `java/java-backend` |
| `ToggleExternalAnnotationsAction` | Toggle External Annotations | `InlayMenu` |  | MISSING | `java/java-backend` |
| `ToggleInferredAnnotationsAction` | Toggle Inferred Annotations | `InlayMenu` |  | MISSING | `java/java-backend` |
| `TurnRefsToSuper` | Use Interface Where Possible… | `RefactoringMenu4` |  | MISSING | `java/java-backend` |
| `UnbindRemoteRepositoryForAllRepositoryLibraries` | Unbind Remote Repositories for All Libraries | `JarRepositoryLibraries` |  | MISSING | `java/java-backend` |
| `UsedIconsListing` | List Used Icons | `Internal.Java` |  | MISSING | `java/xml.dom.impl` |
| `compose.newVectorAsset` | Vector Asset | `NewGroup` |  | MISSING | `plugins/compose` |
| `compose.resource.preview.background.black` | Black | `compose.resource.preview.background` |  | MISSING | `plugins/compose` |
| `compose.resource.preview.background.checkered` | Checkered | `compose.resource.preview.background` |  | MISSING | `plugins/compose` |
| `compose.resource.preview.background.none` | None | `compose.resource.preview.background` |  | MISSING | `plugins/compose` |
| `compose.resource.preview.background.white` | White | `compose.resource.preview.background` |  | MISSING | `plugins/compose` |
| `defaultLombokData` | Default @Data | `LombokActionGroup` |  | MISSING | `plugins/lombok` |
| `defaultLombokEqualsAndHashcode` | Default @EqualsAndHashcode | `LombokActionGroup` |  | MISSING | `plugins/lombok` |
| `defaultLombokGetter` | Default @Getter | `LombokActionGroup` |  | MISSING | `plugins/lombok` |
| `defaultLombokLogger` | @Log (and friends) | `LombokActionGroup` |  | MISSING | `plugins/lombok` |
| `defaultLombokSetter` | Default @Setter | `LombokActionGroup` |  | MISSING | `plugins/lombok` |
| `defaultLombokToString` | Default @ToString | `LombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokAny` | All Lombok Annotations | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokBuilder` | @Builder | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokConstructor` | @Constructors | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokData` | @Data | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokDelegate` | @Delegate | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokEqualsAndHashCode` | @EqualsAndHashCode | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokFieldNameCostants` | @FieldNameConstants | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokGetter` | @Getter | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokLogger` | @Log (and friends) | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokSetter` | @Setter | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokStandardException` | @StandardException | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokSuperBuilder` | @SuperBuilder | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokToString` | @ToString | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokUtilityClass` | @UtilityClass | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokValue` | @Value | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |
| `delombokWither` | @Wither | `DelombokActionGroup` |  | MISSING | `plugins/lombok` |

## Settings pages (7)

| Id | Display name | Group | Parent | Scope | Spec |
|---|---|---|---|---|---|
| `promo.database` | Database | tools |  | application | MISSING |
| `promo.javascript` | JavaScript | language |  | application | MISSING |
| `promo.typescript` | TypeScript | language |  | application | MISSING |
| `promo.swagger` | OpenAPI Specifications | language |  | application | MISSING |
| `compose.resources.settings` | Compose Resources |  | kmp.settings.group | application | MISSING |
| `jvm.logging` | JVM Logging |  | language | project | MISSING |
| `IntelliLang.Advanced` | Advanced |  | IntelliLang.Configuration | project | MISSING |
