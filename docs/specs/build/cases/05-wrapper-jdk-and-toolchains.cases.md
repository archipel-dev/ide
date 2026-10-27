# Cases for build spec 05

Block format as in the build brief.

### BLD-JDK-001 case 1: project macro
settings: default
language: plain
```before
Project SDK = JDK 17 (valid); Gradle JVM = project macro
```
keys: resolve
```after
JDK 17
```

### BLD-JDK-001 case 2: no project sdk
settings: default
language: plain
```before
No project SDK; module SDK JDK 11; table also has JDK 21
```
keys: resolve project macro
```after
JDK 11 (module Java SDK first)
```

### BLD-JDK-001 case 3: table fallback
settings: default
language: plain
```before
No project SDK, no module SDK; table: JDK 11, JDK 21, a broken JDK
```
keys: resolve project macro
```after
JDK 21 (highest valid)
```

### BLD-JDK-001 case 4: internal
settings: default
language: plain
```before
No SDK anywhere
```
keys: resolve project macro
```after
IDE internal runtime
```

### BLD-JDK-001 case 5: JAVA_HOME
settings: default
language: plain
```before
JAVA_HOME=/jdk/17 valid, not in table
```
keys: resolve #JAVA_HOME
```after
synthetic JDK for /jdk/17; unset -> error 'JAVA_HOME environment variable not defined'; invalid -> 'JAVA_HOME environment variable does not point to valid JDK or JRE (/x)'
```

### BLD-JDK-002 case 1: invalid named
settings: default
language: plain
```before
Named JDK 'zulu-17' whose home was deleted
```
keys: resolve
```after
error carrying the path
```

### BLD-JDK-002 case 2: downloading
settings: default
language: plain
```before
JDK entry currently downloading
```
keys: validate
```after
valid (true while downloading)
```

### BLD-JDK-003 case 1: combo
settings: default
language: plain
```before
Open Gradle JVM combo; gradle.properties has org.gradle.java.home=/jdk/17; JAVA_HOME=/jdk/21
```
keys: (open)
```after
Items: project JDK, SDK table JDKs, org.gradle.java.home (Path: /jdk/17), JAVA_HOME (Path: /jdk/21), add/download entries; no 'java.home' item since .gradle/config.properties absent
```

### BLD-JDK-004 case 1: project first
settings: default
language: plain
```before
Project SDK 17 valid
```
keys: pick default
```after
Use Project JDK
```

### BLD-JDK-004 case 2: table highest
settings: default
language: plain
```before
No project SDK; table 11 and 21
```
keys: pick default
```after
name of JDK 21
```

### BLD-JDK-004 case 3: JAVA_HOME
settings: default
language: plain
```before
Empty table; JAVA_HOME valid
```
keys: pick default
```after
Use JAVA_HOME
```

### BLD-JDK-004 case 4: internal
settings: default
language: plain
```before
nothing available
```
keys: pick default
```after
Use Internal JRE
```

### BLD-JDK-010 case 1: reuse
settings: default
language: plain
```before
Another linked project has gradleJvm 'zulu-21'
```
keys: link second project
```after
gradleJvm = zulu-21
```

### BLD-JDK-010 case 2: project sdk
settings: default
language: plain
```before
Project SDK 17, Gradle 8.5
```
keys: link
```after
gradleJvm = Use Project JDK
```

### BLD-JDK-010 case 3: gradle.properties
settings: default
language: plain
```before
No project SDK; gradle.properties org.gradle.java.home=/jdk/17 compatible with Gradle 8.5
```
keys: link
```after
gradleJvm = org.gradle.java.home reference
```

### BLD-JDK-010 case 4: JAVA_HOME
settings: default
language: plain
```before
No project SDK, no property; JAVA_HOME=/jdk/21 compatible
```
keys: link
```after
gradleJvm = Use JAVA_HOME
```

### BLD-JDK-010 case 5: lookup
settings: default
language: plain
```before
Nothing usable locally, Gradle 7.6
```
keys: link
```after
Background lookup, reason text: Download Java to run Gradle 7.6; accepts JDK 8..19 only (IDE minimum 8, Gradle 7.6 matrix up to Java 20 exclusive); result name stored
```

### BLD-JDK-010 case 6: daemon criteria
settings: default
language: plain
```before
gradle-daemon-jvm.properties present, Gradle 8.13
```
keys: link
```after
nothing stored; settings page shows Gradle JVM criteria: group
```

### BLD-JDK-011 case 1: rewrite
settings: default
language: plain
```before
Gradle JVM 'jdk-17'; project SDK later set to jdk-17
```
keys: set project SDK
```after
gradleJvm = Use Project JDK; with Daemon criteria: unchanged
```

### BLD-JDK-012 case 1: resolving
settings: default
language: plain
```before
Lookup running
```
keys: Sync
```after
Status: Gradle JVM is being resolved…; Sync fails with: Gradle JVM is being resolved, please wait.; later: Gradle JVM has been resolved
```

### BLD-JDK-013 case 1: auto jdk
settings: default
language: plain
```before
Selected Gradle JVM unusable for the operation
```
keys: run
```after
Message: It is not possible to use the currently selected Gradle JVM for the operation.
The installation from /jdk/17 will be used instead.
```

### BLD-JDK-020 case 1: java 8 gradle 8
settings: default
language: plain
```before
matrix default
```
keys: isSupported(Gradle 8.14, Java 8)
```after
true (8-9 ↔ INF..9.0.0)
```

### BLD-JDK-020 case 2: java 8 gradle 9
settings: default
language: plain
```before
matrix default
```
keys: isSupported(Gradle 9.0.0, Java 8)
```after
false
```

### BLD-JDK-020 case 3: java 17 gradle 7.1
settings: default
language: plain
```before
matrix default
```
keys: isSupported(Gradle 7.1, Java 17)
```after
false (needs 7.2)
```

### BLD-JDK-020 case 4: java 21 gradle 8.5
settings: default
language: plain
```before
matrix default
```
keys: isSupported(8.5, 21)
```after
true; (8.4, 21) false
```

### BLD-JDK-020 case 5: java 27
settings: default
language: plain
```before
matrix default
```
keys: isSupported(9.8.0, 27); isSupported(9.7.1, 27)
```after
true; false
```

### BLD-JDK-021 case 1: minimums
settings: default
language: plain
```before
IDE data
```
keys: isJavaSupportedByIdea(7); isGradleSupportedByIdea(4.5)
```after
false; false (4.6 true)
```

### BLD-JDK-022 case 1: fallback
settings: default
language: plain
```before
Remote JSON malformed
```
keys: refresh
```after
Shipped default data kept; no error dialog
```

### BLD-JDK-030 case 1: property incompatible
settings: default
language: plain
```before
org.gradle.java.home -> JDK 8 with Gradle 9
```
keys: sync
```after
Notification Found invalid Gradle JVM configuration: JDK 8 isn't compatible with Gradle 9.0.0. Please fix org.gradle.java.home in /w/gradle.properties.
```

### BLD-JDK-030 case 2: env incompatible
settings: default
language: plain
```before
JAVA_HOME -> JDK 8 with Gradle 9
```
keys: sync
```after
JDK 8 isn't compatible with Gradle 9.0.0. Please fix JAVA_HOME environment variable.
```

### BLD-JDK-030 case 3: outcomes
settings: default
language: plain
```before
validation
```
keys: validate undefined / invalid / unsupported / ok
```after
four distinct outcomes
```

### BLD-JDK-031 case 1: wizard
settings: default
language: plain
```before
New project: JDK 25, Gradle 7.6
```
keys: Finish
```after
Unsupported Project JDK: Gradle supports JDK versions 8 through 19.

Do you want to proceed with JDK 25 and Gradle 7.6?
```

### BLD-JDK-040 case 1: no project jdk
settings: default
language: plain
```before
No SDK, strict
```
keys: run task
```after
Project JDK is not specified. Open Project SDK Configuration
```

### BLD-JDK-040 case 2: jre
settings: default
language: plain
```before
Gradle JVM points to a JRE
```
keys: sync
```after
Please use JDK instead of JRE for Gradle importer.
```

### BLD-JDK-040 case 3: platform sdk
settings: default
language: plain
```before
Platform SDK broken
```
keys: run
```after
Platform SDK does not point to valid JDK (/x)
```

### BLD-JDK-040 case 4: maven importer
settings: default
language: plain
```before
Importer JDK 1.7
```
keys: Apply
```after
Maven importer requires JDK 1.8 or higher
```

### BLD-JDK-040 case 5: maven importer changed
settings: default
language: plain
```before
Importer JDK 'gone' removed
```
keys: open
```after
Notification JDK for importer was changed: JDK for importer gone not found and was changed to 17. Consider Maven settings to change importer JDK
```

### BLD-JDK-041 case 1: fix links
settings: default
language: plain
```before
Gradle JVM invalid
```
keys: sync
```after
Quick fix link: Open Gradle Settings
```

### BLD-JDK-042 case 1: unknown sdk
settings: default
language: plain
```before
Project opened with missing SDK
```
keys: (open)
```after
Banner offering to resolve; dialog (build): SDK is not specified or corrupted for module app. Resolve the following SDKs automatically? [Fix automatically]
```

### BLD-JDK-050 case 1: missing wrapper
settings: default
language: plain
```before
Distribution Wrapper, files missing
```
keys: (open page)
```after
'gradle-wrapper.properties' not found
```

### BLD-JDK-050 case 2: update wrapper
settings: default
language: plain
```before
Gradle 7.0 with Java 21
```
keys: click Change Gradle wrapper version to 8.5 and re-sync
```after
distributionUrl rewritten, sync starts; command name Update Gradle Wrapper Version
```

### BLD-JDK-051 case 1: disable notification
settings: default
language: plain
```before
Wrapper source distribution notification
```
keys: disable
```after
Persisted per project; not shown again
```

### BLD-JDK-060 case 1: criteria fields
settings: default
language: plain
```before
Gradle JVM criteria group
```
keys: (look)
```after
Defined criteria is used to auto-detect matching locally installed toolchain or auto-provisioning it for the build execution. Learn more
Version: [Undefined]
Vendor: [Any vendor]
```

### BLD-JDK-060 case 2: validation
settings: default
language: plain
```before
Version Undefined / vendor Custom blank
```
keys: Apply
```after
Please select a version; Please avoid empty or blank value for vendor
```
