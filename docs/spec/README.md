# IntelliJ IDEA reference specification

This directory is the source of truth for what "1:1 with IntelliJ IDEA" means in archipel-ide.
Engineers implement against these files instead of guessing, and the parity catalog in
[`docs/parity/`](../parity/) links back here for the expected behaviour of each feature.

**Target:** IntelliJ IDEA **2026.2** (build 262.x), New UI with the *Islands* themes, on
Windows, macOS and Linux. Java + Gradle is the first product workflow.

## Contents

| File | What it specifies | How it was produced |
|---|---|---|
| [keymaps.md](keymaps.md) | Every default shortcut in the Windows, macOS, GNOME, KDE and XWin keymaps, plus the list of all 26 bundled keymaps and their inheritance | Generated from IntelliJ Community keymap XML and action XML |
| [menus.md](menus.md) | Main menu, main toolbar, editor / project view / tab / tool window context menus, Generate and Refactor This popups, with ids and shortcuts | Generated from action-group XML (3,451 actions, 726 groups) |
| [settings-tree.md](settings-tree.md) | Settings pages registered declaratively, with ids, nesting and project vs application level; full Advanced Settings table with defaults | Generated from `applicationConfigurable` / `projectConfigurable` / `advancedSetting` XML |
| [settings.md](settings.md) | The complete Settings dialog as users see it (including pages registered in code or only in the commercial edition), with options and defaults, and the Project Structure dialog | Researched from public documentation |
| [toolwindows.md](toolwindows.md) | Every tool window, its default stripe and activation shortcut | Generated, plus runtime-registered ones |
| [themes.md](themes.md) | Islands Dark / Islands Light and the other bundled themes: semantic color tokens, component arcs and sizes, editor color schemes and syntax colors | Generated from theme JSON and color scheme XML |
| [ui/workbench.md](ui/workbench.md) | Main window anatomy: main toolbar, stripes, tool window behaviour, editor tabs and gutter, status bar, welcome screen, popups and dialogs | Researched from public documentation |
| [features/platform.md](features/platform.md) | Language-agnostic platform feature inventory with ids, shortcuts and priorities | Researched from public documentation |
| [features/java-gradle.md](features/java-gradle.md) | Java and Gradle feature inventory (editing, navigation, inspections, refactorings, run/debug, Gradle sync and tool window) | Researched from public documentation |
| [data/](data/) | Machine-readable JSON for everything generated: `actions.json`, `keymaps.json`, `settings.json`, `toolwindows.json`, `themes.json`, `color-schemes.json` | Generated |

The JSON files are meant to be consumed directly: the keymap engine (#2) should load
`keymaps.json` rather than re-typing shortcuts, the settings model (#3) should key its pages
on the configurable ids in `settings.json`, and the theme engine should read `themes.json`.

## Regenerating the data

```bash
git clone --depth 1 --branch idea/262.10968.63 --filter=blob:none --no-checkout \
    https://github.com/JetBrains/intellij-community.git /tmp/ic
cd /tmp/ic && git sparse-checkout set --no-cone '*.xml' '**/messages/*.properties' \
    '/platform/platform-resources/src/themes/' '/build.txt' && cd -
python3 -I docs/spec/tools/extract_intellij_data.py /tmp/ic docs/spec
```

When JetBrains ships a new release, bump the tag, regenerate, and review the diff: it is an
exact list of new actions, shortcut changes, new settings and theme changes.

### How the generated data is resolved

- **Keymaps.** A keymap inherits every action it does not list from its `parent`. A keymap
  whose name starts with `Mac OS X` swaps Ctrl and Cmd on shortcuts it inherits from a
  non-macOS parent (this is how `macOS` derives most shortcuts from `$default`). Shortcuts
  declared on actions in plugin XML are then added, removed or replaced per keymap, and
  actions with `use-shortcut-of` borrow another action's shortcuts. The resulting values were
  spot-checked against the published IntelliJ IDEA keymap reference (for example
  Ctrl+N / Cmd+O for Go to Class, Ctrl+Alt+L / Cmd+Alt+L for Reformat Code, Alt+J / Ctrl+G
  for Add Selection for Next Occurrence, KDE's Alt+Shift+L for Reformat Code).
- **Menus.** Groups are expanded in place unless they are popups (submenus). Plugin
  `add-to-group` entries are applied with their `first` / `last` / `before` / `after` anchors;
  anchors that point at actions added by a later plugin are retried until stable. Actions marked
  `internal` are hidden, as in a normal IDE install. A few labels are computed at runtime and
  are filled in from the public documentation (see `RUNTIME_LABELS` in the script).
- **Scope.** Data comes from the open-source IntelliJ Community modules that ship in IntelliJ
  IDEA. Commercial-only features (Spring, Database tools, JavaScript, profiler, ...) appear only
  as "promo" pages in the data; the researched documents cover them where it matters.

## Clean-room rules for this directory

1. Facts only from public sources: the IntelliJ IDEA documentation, the IntelliJ Platform UI
   guidelines, release notes, observable behaviour of the shipped IDE, and Apache-2.0 licensed
   **data files** (XML, JSON, properties) from IntelliJ Community. Attribution is in
   [data/NOTICE.md](data/NOTICE.md).
2. Never copy JetBrains Java/Kotlin source code, icons, screenshots, logos or other artwork into
   this repository. Link to public screenshots and describe them in our own words.
3. Prose is written in our own words. Short UI labels (menu items, setting names, button
   captions) are reproduced exactly because matching them is the point.
4. Anything inferred rather than read from a source is marked "(inferred)"; unresolved
   questions are listed at the end of each researched document so someone can verify them in
   the real IDE.
