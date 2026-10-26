# Search Everywhere, Go to Class/File/Symbol, Recent Files/Locations and the Switcher

Captured with `cargo run -p xtask -- scenes --only 05,06,21,22,23,24,25,26` (Xvfb 1600×1000,
`ARCHIPEL_RENDERER=glow`, java-gradle fixture). PNGs are in `search-everywhere/`.

| Scene | Reference | Capture | Observed |
|---|---|---|---|
| 05 Search Everywhere, `main` | `reference/android-studio/05-search-everywhere.png` | `05-search-everywhere.png` | tabs All / Classes / Files / Symbols / Actions, query field, highlighted matched characters, `file:line` on the right; the only hit for `main` is the `main` method (no Archipel IDE action or file matches), where the reference lists many more kinds of items |
| 06 Find Action, `run` | `reference/android-studio/06-find-action.png` | `06-find-action.png` | Actions tab with category and keymap shortcut per row |
| 21 Go to Class, Ctrl+N, `App` | – | `21-go-to-class.png` | Classes tab, `App` before `AppTest` (shorter name wins the tie), hump highlight |
| 22 Go to File, Ctrl+Shift+N, `app` | – | `22-go-to-file.png` | Files tab |
| 23 Go to Symbol, Ctrl+Alt+Shift+N, `gr` | – | `23-go-to-symbol.png` | Symbols tab (`greeting`, `greets`) |
| 24 Recent Files, Ctrl+E | – | `24-recent-files.png` | newest first, second entry selected |
| 25 Recent Locations, Ctrl+Shift+E | – | `25-recent-locations.png` | line previews with `file:line` |
| 26 Switcher | – | `26-switcher.png` | Files (recent first) and Tool Windows, previous file selected |

## Differences from the reference

- No **Text** tab, "Include non-project items" checkbox, filter or preview buttons.
- Matched characters are tinted and shaded instead of the reference's orange box.
- Class and symbol rows show `file:line` on the right instead of a module name.
- The reference lists settings, layouts and text matches in the All tab; Archipel IDE lists classes,
  files, symbols and actions (four rows each).
- Class/symbol outlines exist for languages whose grammar ships a tags query (Rust, Java).
- Scene 26 opens the Switcher with `navigate.switcher`; holding Ctrl and releasing it to commit
  needs real key-up events, which the control socket's `key` op sends in one step.

## Behavior

- Every shortcut runs through the keymap (`ACTION_COMMANDS`): `SearchEverywhere`, `GotoAction`,
  `GotoClass`, `GotoFile`, `GotoSymbol`, `RecentFiles`, `RecentLocations`, `Switcher` map to
  `navigate.*` commands that only record a request the UI turns into a popup.
- Ranking reuses the completion matcher (`archipel_ide_lsp::completion::fuzzy_match`): exact, prefix,
  camel-hump, then inner matches; shorter names first.
- Symbols come from the file structure (tree-sitter tags outlines) (cached by size and mtime, indexed in 15 ms slices
  per frame while the popup is open; open documents use their live text) plus
  `workspace/symbol` of the running language server (`LspManager::request_workspace_symbols`, a
  new module that leaves the navigation code untouched).
