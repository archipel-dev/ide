# Build spec 05: Wrapper, JDK selection, Gradle JVM resolution and toolchains

Status: specification (clean-room). ID prefix `BLD-JDK`. Executable cases: `cases/05-wrapper-jdk-and-toolchains.cases.md`. The SDK table and project SDK as data are in `docs/specs/platform/07-project-structure-and-scopes.md` (section 1, SDK roots); this spec covers how build tools choose and validate a JDK, the Gradle/Java compatibility matrix, and the messages when the JDK is missing or invalid.

ic: `platform/external-system-impl/src/com/intellij/openapi/externalSystem/service/execution/ExternalSystemJdkUtil.java`, `.../service/execution/` (`ExternalSystemJdkException`, `ProjectJdkNotFoundException`, `InvalidJavaHomeException`, `InvalidSdkException`, `UndefinedJavaHomeException`), `.../service/ui/ExternalSystemJdkComboBox*`, `platform/external-system-api/resources/messages/ExternalSystemBundle.properties` (JDK texts), `plugins/gradle/src/org/jetbrains/plugins/gradle/util/` (`GradleJvmResolutionUtil.kt`, `GradleJvmComboBoxUtil.kt`), `plugins/gradle/src/org/jetbrains/plugins/gradle/resolvers/`, `plugins/gradle/src/org/jetbrains/plugins/gradle/jvmcompat/` (`GradleJvmSupportMatrix`), `plugins/gradle/generated/GradleJvmSupportDefaultData.kt`, `plugins/gradle/src/org/jetbrains/plugins/gradle/properties/` (`GradlePropertiesFile`, `GradleLocalPropertiesFile`), `plugins/gradle/src/org/jetbrains/plugins/gradle/issue/`, `plugins/gradle/resources/messages/GradleBundle.properties`, `java/compiler/impl/src/com/intellij/compiler/impl/CompileDriver.java`, `plugins/maven/src/main/resources/messages/` (`MavenConfigurableBundle`, `MavenSyncBundle`, `MavenRunnerBundle`).

## 1. JDK references

Build tool settings never store a JDK path directly: they store a *reference* string, resolved when needed.

| Reference | Combo item text | Resolves to |
|---|---|---|
| project JDK macro | `Use Project JDK` (shown as the project SDK's name) | the project SDK; if it is a dependent SDK type (one that wraps a real JDK), the underlying registered JDK. If the project has no SDK: the Java SDK of any module; else the highest valid JDK of the SDK table; else the IDE's internal runtime |
| `JAVA_HOME` macro | `Use JAVA_HOME` | the JDK in the environment variable `JAVA_HOME` (an SDK table entry with that path if there is one, otherwise a synthetic JDK for the path) |
| internal macro | `Use Internal JRE` | the runtime the IDE runs on |
| JDK name | the SDK's name | that entry of the SDK table; error if its path is no longer a valid JDK |
| Gradle only: `org.gradle.java.home` item | text `org.gradle.java.home` | the path in `gradle.properties` key `org.gradle.java.home` (build root, Gradle user home or Gradle home), shown only when such a property exists |
| Gradle only: local item | `java.home` | the value of `java.home` in `<build root>/.gradle/config.properties`, shown only when present |
| Gradle only: Daemon JVM criteria | group `Gradle JVM criteria:` | the Gradle daemon JVM selected by Gradle itself from `gradle/gradle-daemon-jvm.properties` |

A JDK is valid iff its home directory passes both the JDK check (contains the compiler tooling) and the JRE check, or if it is currently being downloaded. A non-Java SDK is never valid.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-001 | MUST | Reference kinds and their resolution exactly as the table, including the fallbacks of the project JDK macro. | Cases BLD-JDK-001 case 1..5. |
| BLD-JDK-002 | MUST | JDK validity rule above; a named JDK whose path became invalid throws an error carrying the path (message of BLD-JDK-020). | Cases BLD-JDK-002 case 1..2. |
| BLD-JDK-003 | MUST | The JDK combo box (Gradle JVM, Maven importer JDK, Maven runner JRE): lists the SDK table JDKs, the macro items, the useful references of the table (only those that apply), entries to download or add a JDK (platform SDK combo, labels are an open question); shows the path (`Path: {p}`) and version (`{version}, path: {p}`) as secondary text; while a macro is being resolved the item reads `Resolving…`. | Case BLD-JDK-003 case 1. |

## 2. Available-JDK choice (default for new links)

When the IDE needs a default JDK reference for a project (a new link without stored value), it picks the first that applies:

1. the project SDK, if it is a valid Java SDK compatible with the project (or any module SDK in the same sense): the macro `Use Project JDK`;
2. a JDK that the project SDK refers to (dependent SDK): the macro `Use Project JDK`;
3. the valid JDK of the highest version in the SDK table: that JDK's name;
4. (not in tests) `JAVA_HOME`, when it is a valid JDK;
5. the internal runtime.

For Gradle the order on first link is different and richer (section 3).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-004 | MUST | Default choice order above (used by Maven and generic external systems). | Cases BLD-JDK-004 case 1..4. |

## 3. Gradle JVM resolution

On linking a Gradle project (and when the Gradle version becomes known), unless the project uses Daemon JVM criteria with a Gradle version that supports them (then Gradle chooses and the IDE stores nothing), the Gradle JVM setting is filled as follows, first match wins:

1. an already stored Gradle JVM of any linked Gradle project of this IDE project is reused;
2. the project SDK, if it resolves to a valid JDK: macro `Use Project JDK`;
3. `org.gradle.java.home` of gradle.properties, if present and valid for the Gradle version: the `org.gradle.java.home` reference;
4. `JAVA_HOME`, if it points to a valid JDK compatible with the Gradle version: the `JAVA_HOME` macro;
5. otherwise a background lookup for a JDK that is (a) supported by the IDE (not older than its minimum supported Java) and (b) compatible with the Gradle version by the matrix of section 4, using local JDKs, then downloadable ones; the lookup carries the reason text `Download Java to run Gradle {version}`; the setting is filled when it completes with the found or newly registered JDK's name.

When the project SDK is later set to the JDK the Gradle JVM names explicitly, the Gradle JVM is rewritten to the project-JDK macro (unless Daemon JVM criteria are used). While the Gradle JVM is being resolved: status `Gradle JVM is being resolved…`; an operation that needs it fails with `Gradle JVM is being resolved, please wait.`; completion notice `Gradle JVM has been resolved`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-010 | MUST | Resolution order 1-5 above. | Cases BLD-JDK-010 case 1..6. |
| BLD-JDK-011 | MUST | Project-JDK rewrite rule and Daemon JVM exception. | Case BLD-JDK-011 case 1. |
| BLD-JDK-012 | SHOULD | Status/error texts during asynchronous resolution. | Case BLD-JDK-012 case 1. |
| BLD-JDK-013 | SHOULD | A JDK chosen automatically for the operation because the configured one cannot be used shows `It is not possible to use the currently selected Gradle JVM for the operation.` + `The installation from {path} will be used instead.` | Case BLD-JDK-013 case 1. |

## 4. Gradle / Java compatibility matrix

The matrix maps a range of Java versions to a range of Gradle versions that can run on them. Ranges are `[from, to)` over versions; `INF` means unbounded. Default data (shipped with the IDE; refreshed once a day from the vendor's JSON file, the update interval being 86400 s, falling back to the shipped data):

| Java | Gradle |
|---|---|
| 6 to 8 | INF to 5.0 |
| 8 to 9 | INF to 9.0.0 |
| 9 to 10 | 4.3 to 9.0.0 |
| 10 to 11 | 4.7 to 9.0.0 |
| 11 to 12 | 5.0 to 9.0.0 |
| 12 to 13 | 5.4 to 9.0.0 |
| 13 to 14 | 6.0 to 9.0.0 |
| 14 to 15 | 6.3 to 9.0.0 |
| 15 to 16 | 6.7 to 9.0.0 |
| 16 to 17 | 7.0 to 9.0.0 |
| 17 to 18 | 7.2 to INF |
| 18 to 19 | 7.5 to INF |
| 19 to 20 | 7.6 to INF |
| 20 to 21 | 8.3 to INF |
| 21 to 22 | 8.5 to INF |
| 22 to 23 | 8.8 to INF |
| 23 to 24 | 8.10 to INF |
| 24 to 25 | 8.14 to INF |
| 25 to 26 | 9.1.0 to INF |
| 26 to 27 | 9.4.0 to INF |
| 27 to 28 | 9.8.0 to INF |

IDE-supported Java versions for running Gradle: 8 through 27. IDE-supported Gradle versions: from 4.6 up (list: 4.6, 4.7, 4.8.1, 4.9, 4.10.3, 5.0–5.6.4, 6.0.1–6.9.4, 7.0.2–7.6.6, 8.0.2–8.14.5, 9.0.0–9.8.0). Gradle versions older than the oldest listed (by major.minor) are unsupported; some are deprecated (scheduled for removal). Derived queries: supported Gradle versions for a Java version, supported Java versions for a Gradle version, latest Gradle for a Java, oldest compatible Gradle/Java, recommended Gradle version.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-020 | MUST | `isSupported(gradle, java)` is true iff some table row has java in its Java range and gradle in its Gradle range. | Cases BLD-JDK-020 case 1..5. |
| BLD-JDK-021 | MUST | The IDE rejects Java versions below its minimum supported (8) and Gradle versions below 4.6 (major.minor comparison). | Case BLD-JDK-021 case 1. |
| BLD-JDK-022 | SHOULD | Matrix refresh: shipped default, daily remote update, safe fallback when the download fails or is malformed. | Case BLD-JDK-022 case 1. |

## 5. Validation of Gradle JAVA_HOME properties and notifications

When `org.gradle.java.home` (gradle.properties) or `JAVA_HOME` is used, validation yields success, undefined, invalid, or unsupported (JDK incompatible with the Gradle version). Notification `Found invalid Gradle JVM configuration` (group `Gradle`):

| Case | Text |
|---|---|
| property JDK not compatible | `JDK {j} isn't compatible with Gradle {g}.` then `Please fix org.gradle.java.home in {file}.` |
| env JDK not compatible | `JDK {j} isn't compatible with Gradle {g}.` then `Please fix JAVA_HOME environment variable.` |
| Gradle JVM changed by the IDE | title `Gradle JVM is configured`, text `"{setting}" is set to {value}` + line break + detail |

Wizard-time dialogs (new Gradle project): `Unsupported Project JDK` / `Unsupported Module JDK` with `Gradle supports JDK versions {min} through {max}.` + `Do you want to proceed with JDK {j} and Gradle {g}?`; `Unsupported Gradle Version` with `{IDE} doesn't support Gradle versions older than {v}.`; `Unsupported Java Version` with `{IDE} doesn't support Java versions older than {v} in Gradle projects.`; Kotlin: `Kotlin doesn't support Java versions newer than {v}.` and the dialog variant `Kotlin doesn't support Java versions newer than {v}.` + `Do you want to proceed with Kotlin {k} and JDK {j}?`. In the Gradle settings, an invalid combination shows `JDK {j} isn't supported by Gradle {g}`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-030 | MUST | Validation outcome kinds and the notifications above. | Cases BLD-JDK-030 case 1..3. |
| BLD-JDK-031 | SHOULD | Wizard dialogs above (shown by project creation; the texts are fixed here). | Case BLD-JDK-031 case 1. |

## 6. Messages when the JDK is missing or invalid

| Condition | Text (exact) | Where |
|---|---|---|
| Project has no SDK and a tool needs it | `Project JDK is not specified.` with link `Open Project SDK Configuration` | sync/run failure |
| `JAVA_HOME` not set | `JAVA_HOME environment variable not defined` | sync/run failure |
| `JAVA_HOME` invalid | `JAVA_HOME environment variable does not point to valid JDK or JRE ({path})` | |
| Platform SDK invalid | `Platform SDK does not point to valid JDK ({path})` | |
| Invalid JDK entry | `Invalid Gradle JDK configuration found.`; `Please use JDK instead of JRE for Gradle importer.`; `Gradle JVM option is incorrect:` + `Path:{p}` | |
| Dialog | title `Cannot Use Specified JRE` | |
| Build (IDE builder) | `The SDK is not specified for module X` etc. (BLD-MAKE-026) | |
| Maven importer too old | `Maven importer requires JDK 1.8 or higher`; for Maven 4: `Maven importer requires JDK 17 or higher for Maven 4`; fallback warning `Internal runtime will be used as a fallback. This does not affect project SDK configuration.` | importer settings |
| Maven importer JDK changed | title `JDK for importer was changed`, text `JDK for importer {old} not found and was changed to {new}.` + `Consider Maven settings to change importer JDK` | notification |
| Maven runner JRE | `Cannot find JRE '{x}'` etc. (BLD-MAVEN-062) | |
| Maven build/process JDK requirement | `The build process requires JDK {n} or newer, ...` (BLD-MAKE-026) | |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-040 | MUST | The texts of the table are produced for the conditions of the table and no other texts. | Cases BLD-JDK-040 case 1..5. |
| BLD-JDK-041 | MUST | Errors carry a fix link where listed (Open Gradle settings / Open Project SDK Configuration / Open Importer Settings). | Case BLD-JDK-041 case 1. |
| BLD-JDK-042 | SHOULD | Missing JDK on opening a project (unknown SDK): the platform offers to resolve it automatically (local match or download of a matching JDK) through a banner in the editor and the dialog `SDK is not specified or corrupted for module X. Resolve the following SDKs automatically?` with `Fix automatically` (BLD-MAKE-026); download details (JDK vendor chooser) are an open question. | Case BLD-JDK-042 case 1. |

## 7. Wrapper

Gradle wrapper (`gradle/wrapper/gradle-wrapper.properties`): `Distribution: Wrapper` uses the project's files; when missing, the settings page shows an error. The wrapper task (`Wrapper task` distribution) lets a `wrapper` task in the build script define it. Sync with `Wrapper` downloads the distribution into the Gradle user home if needed (progress `Downloading {x}`), and may report `Cannot resolve Gradle SDK version` / `Invalid Gradle SDK installation` for broken distributions; a quick fix `Change Gradle wrapper version to {v} and re-sync` rewrites `distributionUrl` (command `Update Gradle Wrapper Version`) and re-syncs. `Upgrade Gradle wrapper` is a named execution of the build tool window. `Use these settings for future projects` stores the chosen distribution as default for new projects. Maven wrapper rules are in BLD-MAVEN-002 and BLD-MAVEN-054.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-050 | MUST | Wrapper distribution handling and error texts above; the `Change Gradle wrapper version…` fix and notification group `Gradle wrapper update failed` (BLD-GRADLE-066). | Cases BLD-JDK-050 case 1..2. |
| BLD-JDK-051 | SHOULD | Wrapper source distribution notification can be disabled per project (setting persisted as "disable wrapper source distribution notification"). | Case BLD-JDK-051 case 1. |

## 8. Toolchains

Gradle Daemon JVM criteria (`gradle/gradle-daemon-jvm.properties`: Java version, vendor): group `Gradle JVM criteria:` with title text `Defined criteria is used to auto-detect matching locally installed toolchain or auto-provisioning it for the build execution. Learn more` (link to the Gradle documentation); fields `Version:` (items from the supported Java versions, empty item `Undefined`; errors `Please select a version`, `Invalid version`), `Vendor:` (`Any vendor`, listed vendors, `Custom vendor`; hint `In case vendor isn't specified this wil be resolved based on Gradle rule of precedence`; errors `Please avoid empty or blank value for vendor`, `Invalid vendor`). Issues and fixes: BLD-GRADLE-064; migration offer: BLD-GRADLE-067. Maven toolchains (`toolchains.xml`): BLD-MAVEN-055.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-JDK-060 | SHOULD | Criteria view fields, texts and validation above; applying writes `gradle/gradle-daemon-jvm.properties` and triggers a reload. | Cases BLD-JDK-060 case 1..2. |
| BLD-JDK-061 | LATER | JDK auto-provisioning (download) from the Gradle Daemon criteria and Maven toolchains, vendor-specific installs. Reason: needs the JDK download infrastructure that is outside this area. | n/a |

## Open questions

1. Exact string forms of the macro references stored for the `org.gradle.java.home` and local `java.home` items (`#GRADLE_JAVA_HOME`-style); the resolution rule is specified, the stored literals are not needed for behaviour parity but for project-file compatibility.
2. The JDK download dialog (vendor list, install location) belongs to the platform SDK UI and is not studied here.
3. Whether the Java range upper bound is exclusive in every comparison (the table is written that way from the data file's `from-to` style).
