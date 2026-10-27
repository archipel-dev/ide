# Cases for build spec 04

Block format as in the build brief.

### BLD-MAVEN-001 case 1: general page
settings: default
language: plain
```before
Open Settings | Build Tools | Maven
```
keys: (inspect)
```after
Maven home path: Bundled (Maven 3)
User settings file: ~/.m2/settings.xml [ ] Override
Local repository: ~/.m2/repository [ ] Override
[x] Use settings from .mvn/maven.config
[ ] Show settings dialog for new Maven projects
[ ] Work offline
[x] Execute goals recursively
[ ] Print exception stack traces
[ ] Always update snapshots
Output level: Info
Checksum policy: No Global Policy
Multiproject build fail policy: Default
Thread count:
```

### BLD-MAVEN-001 case 2: flags
settings: default
language: plain
```before
Work offline on, recursively off, stack traces on, update snapshots on, threads 2.0C
```
keys: run goal compile
```after
Command line: compile --offline --non-recursive --errors -U -T 2.0C (flag order not significant)
```

### BLD-MAVEN-002 case 1: bundled items
settings: default
language: plain
```before
Maven home path combo opened
```
keys: (open)
```after
Bundled (Maven 3) | Bundled (Maven 4) | Use Maven wrapper | Specify Maven home…
```

### BLD-MAVEN-002 case 2: invalid home
settings: default
language: plain
```before
Home set to /usr
```
keys: Apply
```after
Invalid Maven home directory (message: Maven home directory is invalid: /usr)
```

### BLD-MAVEN-002 case 3: wrapper
settings: default
language: plain
```before
Use Maven wrapper, .mvn/wrapper/maven-wrapper.properties with distributionUrl https://x/apache-maven-3.9.6-bin.zip
```
keys: sync
```after
Sync tab: Running maven wrapper > Downloading Maven wrapper from https://x/... > Unpacking archive… > Maven wrapper unpacked into <dir>
```

### BLD-MAVEN-003 case 1: override
settings: default
language: plain
```before
Check Override next to Local repository, enter /data/repo
```
keys: Apply
```after
Stored; next sync/run uses -Dmaven.repo.local=/data/repo semantic
```

### BLD-MAVEN-004 case 1: config wins
settings: default
language: plain
```before
.mvn/maven.config contains -o; UI Work offline off; Use settings from .mvn/maven.config on
```
keys: run goal
```after
Offline is applied; UI shows hint: Any changes made in its configuration might be lost after reimporting.
```

### BLD-MAVEN-004 case 2: parse error
settings: default
language: plain
```before
.mvn/maven.config contains broken quote
```
keys: sync
```after
Error parsing '.mvn/maven.config' file: <message>  [Fix maven config]
```

### BLD-MAVEN-010 case 1: importing defaults
settings: default
language: plain
```before
Open Maven | Importing
```
keys: (inspect)
```after
Search for projects recursively [ ]
Use Maven output directories [x]
Exclude build directory (%PROJECT_ROOT%/target) [x]
Use M2E compatibility for Maven plugins [ ]
Generated sources folders: Detect automatically
Phase to be used for folders update: process-resources
Automatically download: Sources [x] Documentation [ ] Annotations [ ]
Keep source and test folders on project reload [x]
Detect compiler automatically [x]
Dependency types: jar, test-jar, maven-plugin, ejb, ejb-client, jboss-har, jboss-sar, war, ear, bundle
VM options for importer:
JDK for importer: Use Project JDK
```

### BLD-MAVEN-011 case 1: dependency types
settings: default
language: plain
```before
Dependency types without 'war'; pom depends on a war artifact
```
keys: sync
```after
War dependency not imported as library
```

### BLD-MAVEN-011 case 2: generated folders
settings: default
language: plain
```before
Generated sources folders: Don't detect; target/generated-sources/x exists
```
keys: sync
```after
x not a source root; with Detect automatically it is
```

### BLD-MAVEN-012 case 1: download
settings: default
language: plain
```before
Automatically download Sources on
```
keys: sync
```after
Background: Downloading Maven library sources…; cancel -> Downloading library components has been canceled; missing: Sources not found for: lib
```

### BLD-MAVEN-013 case 1: runner defaults
settings: default
language: plain
```before
Open Maven | Runner
```
keys: (inspect)
```after
[ ] Delegate IDE build/run actions to Maven
VM Options:
JRE: Use Project JDK
Properties: (No properties defined)
[ ] Skip Tests
```

### BLD-MAVEN-014 case 1: ignored files
settings: default
language: plain
```before
Open Maven | Ignored Files
```
keys: (inspect)
```after
Ignored files: (No Maven files); tooltip: Path patterns (comma-separated, '*' and '?' wildcards allowed):
```

### BLD-MAVEN-020 case 1: pom edit
settings: default
language: plain
```before
Maven project synced; auto-sync External changes
```
keys: edit pom.xml in editor
```after
Floating button: Sync Maven Changes; no auto sync
```

### BLD-MAVEN-020 case 2: jvm.config external
settings: default
language: plain
```before
same
```
keys: edit .mvn/jvm.config on disk
```after
Auto-sync after 3 s
```

### BLD-MAVEN-020 case 3: module pom added
settings: default
language: plain
```before
modules list lists ../m
```
keys: create ../m/pom.xml externally
```after
Tracked (module path); auto-sync
```

### BLD-MAVEN-021 case 1: only changed poms
settings: default
language: plain
```before
Reactor with a/pom.xml, b/pom.xml; a/pom.xml changed
```
keys: reload
```after
Only a is re-read
```

### BLD-MAVEN-021 case 2: non-pom change
settings: default
language: plain
```before
.mvn/maven.config changed
```
keys: reload
```after
Incremental sync of all projects
```

### BLD-MAVEN-022 case 1: profile watcher
settings: default
language: plain
```before
Profile 'dev' activated in the tool window
```
keys: (toggle)
```after
Project marked dirty; reload per auto-sync rule
```

### BLD-MAVEN-030 case 1: off by default
settings: default
language: plain
```before
Maven module, delegation off
```
keys: Ctrl+F9
```after
IDE builder
```

### BLD-MAVEN-030 case 2: on
settings: default
language: plain
```before
Delegate on
```
keys: Ctrl+F9
```after
Run window runs Maven with goals of BLD-MAVEN-031
```

### BLD-MAVEN-031 case 1: incremental
settings: default
language: plain
```before
Delegation on, default phase
```
keys: Ctrl+F9
```after
mvn process-test-classes
```

### BLD-MAVEN-031 case 2: rebuild
settings: default
language: plain
```before
same
```
keys: Rebuild Project
```after
mvn clean process-test-classes
```

### BLD-MAVEN-031 case 3: file compile
settings: default
language: plain
```before
same
```
keys: Recompile file
```after
mvn compile
```

### BLD-MAVEN-031 case 4: two modules
settings: default
language: plain
```before
modules a and b of one reactor, no dependents
```
keys: Build Modules 'a' and 'b'
```after
mvn process-test-classes --projects g:a,g:b (pom of the reactor root)
```

### BLD-MAVEN-032 case 1: skip
settings: default
language: plain
```before
Skip Tests on
```
keys: run delegated build
```after
-DskipTests=true added
```

### BLD-MAVEN-040 case 1: offline toggle
settings: default
language: plain
```before
Click Toggle Offline Mode
```
keys: (click)
```after
Work offline setting on; icon pressed; next goals contain --offline
```

### BLD-MAVEN-040 case 2: skip toggle
settings: default
language: plain
```before
Click Toggle 'Skip Tests' Mode
```
keys: (click)
```after
Runner Skip Tests on
```

### BLD-MAVEN-041 case 1: basic phases
settings: default
language: plain
```before
Project node expanded
```
keys: expand Lifecycle
```after
clean
validate
compile
test
package
verify
install
deploy
site
```

### BLD-MAVEN-041 case 2: all phases
settings: Show Basic Phases Only=off
language: plain
```before
same
```
keys: expand Lifecycle
```after
clean, validate, generate-sources, process-sources, generate-resources, process-resources, compile, process-classes, generate-test-sources, process-test-sources, generate-test-resources, process-test-resources, test-compile, process-test-classes, test, prepare-package, package, pre-integration-test, integration-test, post-integration-test, verify, install, site, deploy
```

### BLD-MAVEN-041 case 3: empty
settings: default
language: plain
```before
No Maven project linked
```
keys: open tool window
```after
There are no Maven projects to display.

You may use <Add Maven Projects> to add a project or <Reload> to let the plugin find all pom.xml files under the project roots and reload them
```

### BLD-MAVEN-042 case 1: toolbar
settings: default
language: plain
```before
Maven tool window
```
keys: (look)
```after
Sync All Maven Projects (split) | Generate Sources and Update Folders For All Projects | Download Sources and/or Documentation | --- | Add Maven Projects | Unlink Maven Projects | --- | Run Maven Build | Execute Maven Goal | Toggle Offline Mode | Toggle 'Skip Tests' Mode | --- | Analyze Maven Dependencies… | --- | Expand All | Collapse All | --- | Build Tools Settings
```

### BLD-MAVEN-043 case 1: run phases
settings: default
language: plain
```before
Select phases clean and package
```
keys: Run Maven Build
```after
Run window: mvn clean package
```

### BLD-MAVEN-043 case 2: execute goal
settings: default
language: plain
```before
click Execute Maven Goal
```
keys: (click)
```after
Input with completion; popup title Maven Goals
```

### BLD-MAVEN-044 case 1: sync vs reload
settings: default
language: plain
```before
Two projects
```
keys: Sync All Maven Projects; then Reload All Maven Projects
```after
first: incremental; second: full re-read (Reload All Maven Projects in popup Sync/Reload All Maven Projects)
```

### BLD-MAVEN-044 case 2: update folders
settings: default
language: plain
```before
Phase process-resources
```
keys: Generate Sources and Update Folders
```after
runs mvn process-resources, then source roots updated
```

### BLD-MAVEN-045 case 1: add invalid file
settings: default
language: plain
```before
Add Maven Projects, choose readme.txt
```
keys: (choose)
```after
Warning dialog Unrecognized Maven Project: Selected file 'readme.txt' isn't Maven file
```

### BLD-MAVEN-045 case 2: add dir without pom
settings: default
language: plain
```before
choose directory empty
```
keys: (choose)
```after
Selected directory '/x' doesn't have Maven files
```

### BLD-MAVEN-045 case 3: unlink child
settings: default
language: plain
```before
Select imported child module
```
keys: Unlink Maven Projects
```after
Failed to remove project: You can not remove child because it's imported as a module of another project parent. You can use Ignore action. Only root project can be removed.
```

### BLD-MAVEN-046 case 1: ignore
settings: default
language: plain
```before
Ignore project b
```
keys: toggle Show Ignored Projects off
```after
b disappears; sync skips b
```

### BLD-MAVEN-047 case 1: profile states
settings: default
language: plain
```before
Profiles node dev
```
keys: click Activate; Deactivate; Use Default
```after
explicit enabled; explicit disabled (-dev); default; Reset Profiles returns all to default
```

### BLD-MAVEN-048 case 1: download selected
settings: default
language: plain
```before
Dependencies node selected
```
keys: Download Sources and Documentation
```after
sources+docs fetched with progress
```

### BLD-MAVEN-049 case 1: effective pom
settings: default
language: plain
```before
project node
```
keys: Show Effective POM
```after
Editor with effective POM; failure: Error / Failed to evaluate effective pom.
```

### BLD-MAVEN-050 case 1: sync steps
settings: default
language: plain
```before
Open Maven project
```
keys: sync
```after
Sync tab: Scanning Maven projects… > Reading Maven projects… > Resolve plugins > Resolve dependencies > Generating Maven sources… > Importing Maven projects > Configuring Maven projects
```

### BLD-MAVEN-051 case 1: problems
settings: default
language: plain
```before
pom with <parent> not found and missing module
```
keys: sync
```after
Errors: Parent 'g:p:1' not found ; Module 'x' not found (with pom file location)
```

### BLD-MAVEN-052 case 1: no maven
settings: default
language: plain
```before
No Maven home, M2_HOME unset
```
keys: run goal
```after
No valid Maven installation found. Either set the home directory in the configuration dialog or set the M2_HOME environment variable on your system.
```

### BLD-MAVEN-052 case 2: not exist
settings: default
language: plain
```before
home /gone
```
keys: run
```after
Specified Maven home directory (/gone) does not exist. [Configure Maven home]
```

### BLD-MAVEN-052 case 3: maven4 jdk
settings: default
language: plain
```before
Maven 4, JDK 11
```
keys: sync
```after
Apache Maven 4.X Requires Java 17 or Newer to Run. [Choose another JDK for Importer and Runner…]
```

### BLD-MAVEN-052 case 4: too low
settings: default
language: plain
```before
Maven 3.9 with JDK 1.6
```
keys: sync
```after
Maven JDK Version for Importer Is Too Low: For Maven 3.9 at least JDK 1.8 is required. [Choose another JDK for Importer and Runner…] [Open Importer Settings…]
```

### BLD-MAVEN-053 case 1: http blocked
settings: default
language: plain
```before
pom declares http repository with Maven 3.8.1+
```
keys: sync
```after
Since Maven 3.8.1 http repositories are blocked
Possible solutions:
 - Check that Maven settings.xml does not contain http repositories
 - Check that Maven pom files do not contain http repository <url>
 - Add a mirror(s) for <url> that allows http url in the Maven settings.xml
 - Downgrade Maven to version 3.8.1 or earlier in settings
```

### BLD-MAVEN-054 case 1: wrapper empty url
settings: default
language: plain
```before
maven-wrapper.properties without distributionUrl
```
keys: sync
```after
Notification Maven wrapper: DistributionUrl is empty. The Maven bundled version will be used instead.
```

### BLD-MAVEN-055 case 1: language level
settings: default
language: plain
```before
pom without release level, JDK 21
```
keys: sync
```after
Language level is invalid or missing in pom.xml. Current project JDK is 21. Specify language level in pom.xml
```

### BLD-MAVEN-056 case 1: untrusted
settings: default
language: plain
```before
Untrusted project
```
keys: open
```after
Sync tab: Untrusted project — The opened untrusted project is not fully loaded. IntelliJ IDEA will not resolve any plugins in the project, download artifacts, or execute any goals [Trust project]
```

### BLD-MAVEN-057 case 1: wsl
settings: default
language: plain
```before
WSL project with Windows JDK
```
keys: sync
```after
Windows JDK used for WSL Project (platform-specific, LATER)
```

### BLD-MAVEN-060 case 1: editor
settings: default
language: plain
```before
Create Maven run configuration
```
keys: (look)
```after
Working directory: <pom dir>
Command line: (empty text: Command line)
Maven options group (Inherit from settings)
Java options group: VM options, JRE
```

### BLD-MAVEN-060 case 2: override
settings: default
language: plain
```before
Profiles set to 'dev' in configuration only
```
keys: run
```after
-P dev only for this configuration
```

### BLD-MAVEN-060 case 3: properties
settings: default
language: plain
```before
Properties dialog
```
keys: add k=v
```after
Saved; command line has -Dk=v
```

### BLD-MAVEN-061 case 1: assembly
settings: default
language: plain
```before
Command line 'clean install', profiles 'dev -test', Skip Tests
```
keys: run
```after
mvn clean install -P dev,-test -DskipTests=true
```

### BLD-MAVEN-061 case 2: quiet
settings: default
language: plain
```before
Output level Error
```
keys: run
```after
-q appended
```

### BLD-MAVEN-061 case 3: threads
settings: default
language: plain
```before
Thread count 1C
```
keys: run
```after
-T 1C
```

### BLD-MAVEN-062 case 1: messages
settings: default
language: plain
```before
Goal fails
```
keys: run
```after
Output ends: BUILD FAILED; status line: Maven execution terminated abnormally (aborted: Maven execution aborted; success: Maven execution finished)
```

### BLD-MAVEN-063 case 1: before launch
settings: default
language: plain
```before
Goal 'generate-sources' before Run
```
keys: Run
```after
Before launch row: Run Maven Goal 'generate-sources'; progress text: generate-sources (1 of 2)
```

### BLD-MAVEN-064 case 1: run anything
settings: default
language: plain
```before
Run Anything
```
keys: type mvn clean
```after
Maven goal completion and run in project context
```

### BLD-MAVEN-070 case 1: repos
settings: default
language: plain
```before
Open Remote Jar Repositories
```
keys: (inspect)
```after
List with Maven Central and bundled repositories; buttons add, edit, remove
```
