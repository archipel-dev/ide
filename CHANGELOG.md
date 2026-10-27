# Changelog

## [0.2.2](https://github.com/archipel-dev/ide/compare/v0.2.1...v0.2.2) (2026-10-27)


### Features

* **java:** native Java resolve, unresolved-symbol errors and IntelliJ-style import fixes ([#57](https://github.com/archipel-dev/ide/issues/57)) ([956fc07](https://github.com/archipel-dev/ide/commit/956fc076c6fb18c1c51dc65e3ed965a7b78696be))
* **xtask:** behavior scenario runner and a 32-scenario IntelliJ parity corpus ([#58](https://github.com/archipel-dev/ide/issues/58)) ([3aa4430](https://github.com/archipel-dev/ide/commit/3aa4430e5948f606074c1e157f1f9b7f4be59fc8))
* **xtask:** spec coverage report for the clean-room requirements ([#60](https://github.com/archipel-dev/ide/issues/60)) ([b5ede59](https://github.com/archipel-dev/ide/commit/b5ede59d81076ecf6eabb1e6dcd479e99b2ecbbb))

## [0.2.1](https://github.com/archipel-dev/ide/compare/v0.2.0...v0.2.1) (2026-10-27)


### Features

* **projectmodel:** project model and External System import for Gradle, Maven, Cargo and Bazel ([#50](https://github.com/archipel-dev/ide/issues/50)) ([ba7eab5](https://github.com/archipel-dev/ide/commit/ba7eab591ff424bc61d7a7477b00a88067add0b7))


### Bug Fixes

* **daemon:** LSP diagnostics as an external pass, one Alt+Enter list, working jdtls static imports ([#52](https://github.com/archipel-dev/ide/issues/52)) ([2fe93df](https://github.com/archipel-dev/ide/commit/2fe93dfbd1c04b215d333777faa48301d42b6e35))
* **toolchains:** run jdtls on a JDK as new as its bundles need ([#53](https://github.com/archipel-dev/ide/issues/53)) ([54dca44](https://github.com/archipel-dev/ide/commit/54dca44f98af50dabcecf88debd745a66ccb1118))

## [0.2.0](https://github.com/archipel-dev/ide/compare/v0.1.0...v0.2.0) (2026-10-26)


### ⚠ BREAKING CHANGES

* move the IDE onto the Archipel Kit IntelliJ Platform names ([#45](https://github.com/archipel-dev/ide/issues/45))

### Features

* **toolchains:** set up missing tools on project open, fix the welcome search and New Project layout ([#46](https://github.com/archipel-dev/ide/issues/46)) ([c02fd20](https://github.com/archipel-dev/ide/commit/c02fd20d290e74207896ea5140444117513eff12))


### Code Refactoring

* move the IDE onto the Archipel Kit IntelliJ Platform names ([#45](https://github.com/archipel-dev/ide/issues/45)) ([2bcd76e](https://github.com/archipel-dev/ide/commit/2bcd76e5e054bc39e0867f03044ec7d8cf1f24a4))

## [0.1.0](https://github.com/archipel-dev/ide/compare/v0.0.9...v0.1.0) (2026-10-26)


### ⚠ BREAKING CHANGES

* rename Jetthing to Archipel IDE ([#34](https://github.com/archipel-dev/ide/issues/34))

### Features

* **analysis:** analysis pipeline contracts and core (wave 0 + WP1) ([#36](https://github.com/archipel-dev/ide/issues/36)) ([56592a4](https://github.com/archipel-dev/ide/commit/56592a4222e95f0ebc64d70e5a56e245ba57fd70))
* **daemon:** highlighting daemon, inspections and intentions (WP3) ([#40](https://github.com/archipel-dev/ide/issues/40)) ([31c0a80](https://github.com/archipel-dev/ide/commit/31c0a80b67fc1e62988cd3adb149dac151600d6f))
* **git:** worktree management for agent sessions ([#38](https://github.com/archipel-dev/ide/issues/38)) ([5db5c2a](https://github.com/archipel-dev/ide/commit/5db5c2af36de684b5f078f52e605092fcd204123))
* **index:** persistent indexes, dumb mode, TODO and Local History (WP2) ([#39](https://github.com/archipel-dev/ide/issues/39)) ([d892d61](https://github.com/archipel-dev/ide/commit/d892d61ed924c5e95dc90439bfa7444f1b225d6f))
* **project:** Spring Boot initializr wizard with Gradle, Maven and Bazel ([#44](https://github.com/archipel-dev/ide/issues/44)) ([8e8293c](https://github.com/archipel-dev/ide/commit/8e8293c4c800329c7ed83efadc4a83a567b5f833))
* **ui:** agents mode prototype with a mocked backend ([#41](https://github.com/archipel-dev/ide/issues/41)) ([e8b6866](https://github.com/archipel-dev/ide/commit/e8b6866dd4605e780e53f8434fb2c4ff019c4e29))
* **ui:** build the UI foundation on archipel-kit ([#28](https://github.com/archipel-dev/ide/issues/28)) ([496e652](https://github.com/archipel-dev/ide/commit/496e652d9e9288a3fd70a14e0ef3bffa9c5ebe5f))


### Code Refactoring

* rename Jetthing to Archipel IDE ([#34](https://github.com/archipel-dev/ide/issues/34)) ([4aa1b4b](https://github.com/archipel-dev/ide/commit/4aa1b4b8377abd5886a96ebf34eefef5fca2be4f))

## [0.0.9](https://github.com/HenrySchwerdt/jetthing/compare/v0.0.8...v0.0.9) (2026-10-11)

v0.0.8 was published without downloads; this release ships its contents plus the changes below.

### Features

* **java:** Generate popup (Alt+Insert / ⌘N), Go to Super Method, Go to Test / Create Test, and a Hierarchy tool window for type and call hierarchies ([#216](https://github.com/HenrySchwerdt/jetthing/pull/216))

* **editor:** splits (Split Right / Down, drag a tab to an editor edge, Move to Opposite Group, Unsplit), Open in Right Split (Shift+Enter) and an italic preview tab on single click ([#217](https://github.com/HenrySchwerdt/jetthing/pull/217))

### Bug Fixes

* **ui:** pointer cursor on buttons; the editor shows the I-beam over text only, the gutter keeps the arrow ([#215](https://github.com/HenrySchwerdt/jetthing/pull/215))
* **ui:** hover and pressed states in the gutter Run popup; Escape and a click outside close it ([#215](https://github.com/HenrySchwerdt/jetthing/pull/215))
* **editor:** multi-line selections are drawn as one shape, rounded only at their outer corners ([#215](https://github.com/HenrySchwerdt/jetthing/pull/215))
* **ci:** the release job no longer deletes the built installers before uploading them, and fails when none are found

## [0.0.8](https://github.com/HenrySchwerdt/jetthing/compare/v0.0.7...v0.0.8) (2026-10-11)

### Features

* **plugins:** bundle nexus-http as a default plugin with a Tools menu entry, standalone app and `.http` editor ([#208](https://github.com/HenrySchwerdt/jetthing/pull/208))
* **editor:** Fold Selection (Ctrl+.) ([#209](https://github.com/HenrySchwerdt/jetthing/pull/209))
* **search:** user-defined Find in Files scopes ([#209](https://github.com/HenrySchwerdt/jetthing/pull/209))
* **git:** open file history as a sibling tab of the Log tab ([#209](https://github.com/HenrySchwerdt/jetthing/pull/209))
* **diff:** collapse-unchanged and sync-scroll toggles ([#209](https://github.com/HenrySchwerdt/jetthing/pull/209))
* **lsp:** accept completions on commit characters ([#209](https://github.com/HenrySchwerdt/jetthing/pull/209))
* **runner:** JDK row on the Application run configuration ([#209](https://github.com/HenrySchwerdt/jetthing/pull/209))

### Bug Fixes

* **ui:** keep the New Project dialog's Create/Cancel buttons visible and widen the GroupId field ([#211](https://github.com/HenrySchwerdt/jetthing/pull/211))

### Miscellaneous

* **ci:** release-please for automatic versioning ([#206](https://github.com/HenrySchwerdt/jetthing/pull/206))
* **docs:** parity table refresh and Java debugger screenshot in the README ([#207](https://github.com/HenrySchwerdt/jetthing/pull/207), [#210](https://github.com/HenrySchwerdt/jetthing/pull/210))
