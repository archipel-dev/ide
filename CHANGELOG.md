# Changelog

## [0.0.10](https://github.com/archipel-dev/ide/compare/v0.0.9...v0.0.10) (2026-10-11)


### Features

* **ui:** build the UI foundation on archipel-kit ([#28](https://github.com/archipel-dev/ide/issues/28)) ([496e652](https://github.com/archipel-dev/ide/commit/496e652d9e9288a3fd70a14e0ef3bffa9c5ebe5f))

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
