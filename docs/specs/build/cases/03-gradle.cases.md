# Cases for build spec 03

Block format as in the build brief.

### BLD-GRADLE-001 case 1: page layout
settings: default
language: plain
```before
Open Settings | Build Tools | Gradle for a linked project
```
keys: (inspect)
```after
[ ] Download external annotations for dependencies
Build and Run (hint text)
  Build and run using: [Gradle (Default)]
  Run tests using: [Gradle (Default)]
Gradle
  Distribution: [Wrapper]
  Gradle JVM: [Project SDK ...]
Gradle user home: <placeholder ~/.gradle>
Gradle VM options:
[x] Generate *.iml files for modules imported from Gradle
(module-per-source-set checkbox hidden because it is on)
```

### BLD-GRADLE-002 case 1: defaults
settings: default
language: plain
```before
New linked project
```
keys: (read settings)
```after
gradleJvm=#USE_PROJECT_JDK distribution=DEFAULT_WRAPPED delegatedBuild=true testRunner=GRADLE modulePerSourceSet=true externalAnnotations=false offline=false
```

### BLD-GRADLE-003 case 1: local empty
settings: default
language: plain
```before
Distribution: Local installation, location empty
```
keys: Apply
```after
Error: Gradle location is not specified
```

### BLD-GRADLE-003 case 2: local invalid
settings: default
language: plain
```before
Distribution: Local installation, location /nope
```
keys: Apply
```after
Error: Gradle location is incorrect.
Location:/nope
```

### BLD-GRADLE-003 case 3: wrapper missing
settings: default
language: plain
```before
Distribution Wrapper; no gradle/wrapper/gradle-wrapper.properties
```
keys: (open page)
```after
Error icon with text: 'gradle-wrapper.properties' not found
```

### BLD-GRADLE-003 case 4: jvm unset
settings: default
language: plain
```before
Gradle JVM empty
```
keys: Apply
```after
Error: Please set the Gradle JVM option
```

### BLD-GRADLE-004 case 1: combos
settings: default
language: plain
```before
Open Build and Run group
```
keys: open both combos
```after
Build and run using: Gradle (Default), IntelliJ IDEA
Run tests using: Gradle (Default), IntelliJ IDEA, Choose per test
```

### BLD-GRADLE-005 case 1: user home
settings: default
language: plain
```before
GRADLE_USER_HOME=/data/gh
```
keys: (open page)
```after
Placeholder shows /data/gh; sync uses that directory for caches.
```

### BLD-GRADLE-006 case 1: migrate
settings: default
language: plain
```before
Gradle VM options '-Xmx2g' set
```
keys: click Migrate
```after
Confirm: Would you like to move VM options to the '/work/app/gradle.properties' file? / Note that the existing 'org.gradle.jvmargs' property will be overwritten. / You can do it manually any time later
After OK: field text 'VM options have been moved to gradle.properties'
```

### BLD-GRADLE-007 case 1: checkboxes
settings: default
language: plain
```before
Parallel fetch off
```
keys: tick Enable parallel Gradle model fetching for Gradle 7.4+
```after
Hint shown: Allow collecting Gradle models in parallel during a project reload. Warning: Incubating. Non-thread safe access to shared Gradle configuration data in build.gradle may cause concurrency issues.
```

### BLD-GRADLE-008 case 1: dirty on source set change
settings: default
language: plain
```before
Module-per-source-set toggled off (deprecated option visible)
```
keys: Apply
```after
Project marked dirty: floating button Sync Gradle Changes appears (or auto-reload per setting)
```

### BLD-GRADLE-009 case 1: advanced
settings: default
language: plain
```before
Advanced Settings page
```
keys: search Build Tools. Gradle
```after
Run using Gradle [x]; Download sources [x]; Attach scripts dependencies sources [ ]
```

### BLD-GRADLE-020 case 1: properties
settings: default
language: plain
```before
Gradle project synced
```
keys: edit <gradle user home>/gradle.properties externally
```after
Project stale (external); auto-sync after 3 s
```

### BLD-GRADLE-020 case 2: catalog
settings: default
language: plain
```before
gradle/libs.versions.toml imported
```
keys: edit it in the editor (External changes mode)
```after
Stale; Sync Gradle Changes button shown
```

### BLD-GRADLE-020 case 3: wrapper properties
settings: default
language: plain
```before
Wrapper distribution
```
keys: edit distributionUrl in gradle/wrapper/gradle-wrapper.properties
```after
Stale. With distribution Local installation the same edit is ignored.
```

### BLD-GRADLE-021 case 1: mapping
settings: default
language: plain
```before
Modules /w/app and /w/app/sub; /w/app/build is a compile output
```
keys: edit /w/app/sub/build.gradle; edit /w/app/build/x.gradle; edit /w/app/readme.txt
```after
first: maps to project /w/app; second and third: not tracked
```

### BLD-GRADLE-030 case 1: delegated
settings: default
language: plain
```before
Gradle module, Build and run using = Gradle
```
keys: Ctrl+F9
```after
Gradle runs; tab 'Build app'
```

### BLD-GRADLE-030 case 2: IDE
settings: default
language: plain
```before
Build and run using = IntelliJ IDEA
```
keys: Ctrl+F9
```after
IDE builder runs; no Gradle process
```

### BLD-GRADLE-030 case 3: run only
settings: default
language: plain
```before
Advanced setting Run using Gradle = off, Build and run using = Gradle
```
keys: Shift+F10 on an application configuration
```after
The application runs through the IDE, build still delegated to Gradle
```

### BLD-GRADLE-031 case 1: main sourceset
settings: default
language: plain
```before
Module app.main, tasks include classes
```
keys: Ctrl+F9
```after
Gradle executed with task ':classes' (in root build 'Build <folder>')
```

### BLD-GRADLE-031 case 2: test sourceset
settings: default
language: plain
```before
Module app.test
```
keys: build module
```after
Task ':testClasses'
```

### BLD-GRADLE-031 case 3: fallback assemble
settings: default
language: plain
```before
main source set without classes task
```
keys: build module
```after
Task ':assemble'
```

### BLD-GRADLE-031 case 4: rebuild
settings: default
language: plain
```before
Rebuild of module app
```
keys: Ctrl+Shift+F9
```after
Same tasks plus init script making compile tasks never up to date
```

### BLD-GRADLE-032 case 1: serialize and refresh
settings: default
language: plain
```before
Two builds requested
```
keys: Ctrl+F9 twice
```after
second waits; after finish output roots refreshed; failure activates the Build window
```

### BLD-GRADLE-040 case 1: toolbar
settings: default
language: plain
```before
Gradle tool window
```
keys: (look)
```after
Toolbar: Sync All Gradle Projects, Select Project Data To Import, Download Sources, Unlink Gradle Project, Execute Gradle Task, Toggle Offline Mode, Composite Build Configuration, Analyze Gradle Dependencies…, Expand All, Collapse All, settings
```

### BLD-GRADLE-041 case 1: run anything prefix
settings: default
language: plain
```before
Run Anything popup
```
keys: type: gradle
```after
Help row: gradle <taskName...> <--option-name...>
```

### BLD-GRADLE-041 case 2: order after dash
settings: default
language: plain
```before
Tasks: build, :app:test; options: --info, -i
```
keys: type: gradle -
```after
Variants order: task options, short options (-i), long options (--info), class args, wildcard tasks (build), path tasks (:app:test)
```

### BLD-GRADLE-041 case 3: order after colon
settings: default
language: plain
```before
same
```
keys: type: gradle :
```after
Variants order: :app:test first, then build, then options
```

### BLD-GRADLE-041 case 4: run
settings: default
language: plain
```before
Project context
```
keys: type: gradle build --info, Enter
```after
Temporary Gradle run configuration 'build --info' (or equal existing selected) runs
```

### BLD-GRADLE-042 case 1: bad args
settings: default
language: plain
```before
Run Anything
```
keys: gradle --no-such-flag
```after
Balloon (warning): Command-line arguments cannot be parsed
<i>--no-such-flag</i> + parser message; nothing runs
```

### BLD-GRADLE-043 case 1: editor
settings: default
language: plain
```before
Create Gradle run configuration
```
keys: (look)
```after
Gradle project: /w/app
Tasks and arguments: [empty text: Tasks and arguments]  hint: Example: build --debug. Default tasks will be executed if no tasks are specified.
VM options / Environment variables
Debug Gradle scripts, Run as test (comment: Shows test view and forces test tasks re-execution)
```

### BLD-GRADLE-043 case 2: empty project
settings: default
language: plain
```before
Gradle project field empty
```
keys: Apply
```after
Error: Gradle project is not specified
```

### BLD-GRADLE-043 case 3: split storage
settings: default
language: plain
```before
Command line 'clean build --info'
```
keys: save
```after
tasks=[clean, build]; script parameters='--info'
```

### BLD-GRADLE-044 case 1: offline on
settings: default
language: plain
```before
Toggle Offline Mode on
```
keys: run task build
```after
Command line contains --offline; sync also offline
```

### BLD-GRADLE-044 case 2: offline failure
settings: default
language: plain
```before
offline on; dependency missing
```
keys: sync
```after
Quick fix link: Disable offline mode and reload the project
```

### BLD-GRADLE-045 case 1: search
settings: default
language: plain
```before
Search Everywhere Actions
```
keys: type Toggle Gradle Offline
```after
Entry: Toggle Gradle Offline Mode
```

### BLD-GRADLE-046 case 1: sources failure
settings: default
language: plain
```before
Download Sources, repo lacks sources
```
keys: (run)
```after
Sticky balloon (group Gradle): Sources download failed — Sources were not downloaded for: lib-1.0
```

### BLD-GRADLE-047 case 1: names
settings: default
language: plain
```before
Run test class in module app.test
```
keys: right-click run
```after
Configuration name: test in app (task first form: {task} in {module}); several patterns: test and 2 more
```

### BLD-GRADLE-050 case 1: gradle runner
settings: default
language: plain
```before
Run tests using = Gradle; class FooTest
```
keys: Ctrl+Shift+F10
```after
Gradle config: test --tests 'FooTest'; Run window shows test tree
```

### BLD-GRADLE-050 case 2: platform
settings: default
language: plain
```before
Run tests using = IntelliJ IDEA
```
keys: Ctrl+Shift+F10
```after
JUnit run configuration, IDE runner
```

### BLD-GRADLE-050 case 3: choose
settings: default
language: plain
```before
Choose per test, first run
```
keys: Ctrl+Shift+F10
```after
Popup: Run Tasks for app / Select several tasks to run them at once  (or 'No tasks available' warning when none)
```

### BLD-GRADLE-051 case 1: no events
settings: default
language: plain
```before
Gradle test run ends without test events
```
keys: (run)
```after
Tree node: Test events were not received
```

### BLD-GRADLE-052 case 1: report
settings: default
language: plain
```before
Finished test run
```
keys: Open Gradle test report
```after
HTML report from build/reports/tests opens
```

### BLD-GRADLE-054 case 1: composite
settings: default
language: plain
```before
Project with included build ../lib
```
keys: Composite Build Configuration
```after
Dialog text: Please select participant projects to include in the Gradle composite.; [Select All] [Unselect All]; tree lists lib
```

### BLD-GRADLE-055 case 1: orphans
settings: default
language: plain
```before
Gradle 3.2 composite
```
keys: sync
```after
Gradle project structure problems: Can't find the following module: lib ...
```

### BLD-GRADLE-056 case 1: source sets
settings: default
language: plain
```before
app with main and test
```
keys: sync
```after
Modules: app, app.main, app.test
```

### BLD-GRADLE-057 case 1: gutter
settings: default
language: plain
```before
build.gradle: task hello {}
```
keys: (look)
```after
Green run icon at the task line; click: Run 'hello', Debug 'hello', ...
```

### BLD-GRADLE-060 case 1: incompatible
settings: default
language: plain
```before
Gradle 7.2, Java 21
```
keys: sync
```after
Incompatible Gradle JVM
Your build is currently configured to use incompatible Java 21 and Gradle 7.2. Cannot sync the project.
The minimum compatible Gradle version is 8.5.
Use Java 17 as Gradle JVM: Open Gradle settings
Change Gradle wrapper version to 8.5 and re-sync
```

### BLD-GRADLE-060 case 2: unknown java
settings: default
language: plain
```before
unknown Java
```
keys: sync
```after
Your build is currently configured to use incompatible Java. Cannot sync the project.
```

### BLD-GRADLE-061 case 1: unsupported jvm
settings: default
language: plain
```before
Java 5
```
keys: sync
```after
Unsupported Gradle JVM
Java 5 is not supported for Gradle execution. Cannot sync the project.
```

### BLD-GRADLE-062 case 1: gradle old
settings: default
language: plain
```before
Gradle 3.0
```
keys: sync
```after
Unsupported Gradle Version
Gradle 3.0 is not supported. Cannot sync the project.
We recommend upgrading to Gradle version <x>.
```

### BLD-GRADLE-063 case 1: bad jdk
settings: default
language: plain
```before
Gradle JVM points to missing dir
```
keys: sync
```after
Gradle JVM option is incorrect
Unable to use JDK from /bad with project located in /w/app.
Open Gradle Settings
```

### BLD-GRADLE-064 case 1: toolchain
settings: default
language: plain
```before
Daemon JVM criteria version 21, no toolchain, download fails
```
keys: sync
```after
Unable to Download Toolchain: Failed to locate and download a Daemon JVM with matching criteria (version=21, vendor=Any). Consider installing it manually or modifying Daemon JVM criteria.
```

### BLD-GRADLE-065 case 1: unresolved
settings: default
language: plain
```before
dependency com.x:y:1 missing
```
keys: sync
```after
Could Not Resolve com.x:y:1 for :app
Declare repository providing the artifact, see the documentation at <url>
```

### BLD-GRADLE-066 case 1: wrapper fail
settings: default
language: plain
```before
Quick fix changes wrapper version but write fails
```
keys: click fix
```after
Notification: Unable to update wrapper files / See IDE log for the details.
```

### BLD-GRADLE-067 case 1: migrate notification
settings: default
language: plain
```before
Project eligible
```
keys: (open)
```after
Notification Migrate to Gradle Daemon toolchain with actions Migrate, Ignore, Learn more
```

### BLD-GRADLE-070 case 1: analyzer
settings: default
language: plain
```before
Gradle project
```
keys: Analyze Gradle Dependencies…
```after
Dependency Analyzer tab; busy text Load Gradle Dependencies
```

### BLD-GRADLE-071 case 1: titles
settings: default
language: plain
```before
Running a task
```
keys: (watch progress)
```after
Progress titles in order: Configuring build, Configuring projects, Running tasks
```
