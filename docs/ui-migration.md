# Migrating the IDE UI to archipel-kit

The IDE's panels were built one at a time, each with its own padding, icon alignment, hover and
cursor behaviour and popup frame. [archipel-kit](https://github.com/archipel-dev/kit) fixes that
once for the whole Archipel suite: one `Theme`, one token set, one icon pack, components with
one set of states, and cursor rules. This guide is for the workers who move one area at a time.

Read the kit docs first (`docs/README.md`, `getting-started.md`, `theming.md`, `cursors.md`,
`behaviours.md`, `building-components.md`, `contributing.md` in the kit repository).

## Rules

1. **No new ad-hoc egui painting in `jetthing-ui`.** No new `painter().rect_filled`, hand-made
   hover fills, hand-rolled icon buttons, popup frames or cursor code. Use a kit component. If the
   kit lacks something, add it to the kit (a PR there, with its tests, docs and gallery page) or
   build it from kit parts as described in the kit's `building-components.md` (`State`,
   `RowContent`, `Surface`, `Popover`, `Slot`, behaviours). Then use it here.
2. Colors come from `archipel_kit::Theme::colors(ctx)` or a `Tone`; sizes come from
   `archipel_kit::tokens`. The old `theme::TEXT`-style tokens still work (they resolve to the
   kit's values) but new code should not add more call sites.
3. UI controls dispatch commands; they do not own project policy (see `AGENTS.md`). Migrating
   a panel changes how it looks and behaves, not which command it runs.
4. Every migrated area gets an audit test (below). A panel that fails the audit is not done.

## What the foundation PR did

| Piece | Where | State |
|---|---|---|
| Dependencies | `Cargo.toml` (`archipel-kit`, `archipel-icons`, git, branch `feat/components-v0.1` until kit PR #1 merges, then switch to `main`) | egui/eframe 0.33, same as the kit. Engine crates stay egui-free. |
| Theme | `theme_engine.rs`, `theme.rs` | `theme::install` calls `archipel_kit::install`. Each palette (theme + editor scheme, switched at runtime in Settings) builds one `archipel_kit::Theme`; `Theme::apply` runs on every switch, so kit components and the IDE's old tokens always agree. Workbench tokens take the kit's value; the editor tokens follow the editor color scheme and feed the kit's `EditorColors`. |
| Layout tokens | `theme::layout` | Re-exports of `archipel_kit::tokens::size`. |
| Icons | `icons.rs`, `file_icon.rs` | `icons::Icon` is an adapter: every variant maps to a kit `Glyph` (`Icon::kit()`), file icons come from `archipel_kit::Icon::for_file`, `icons::paint/show/button` delegate to the kit. |
| Frames | `widgets::island_frame(ctx)`, `chrome_frame(ctx)`, `popups::popup_frame` | `Surface::Island` (without its inner padding), the kit frame color, `Surface::Dialog`. |
| Rows | `widgets::list_row`, `widgets::tree_row` | Kit `ListRow` (and kit parts where the caller paints the content itself). |
| Menus | `widgets::menu_item`, `context_menus::show` | Kit `MenuItem`, `MenuSeparator`, `ContextMenu`. |
| Fields | `widgets::search_field` | The kit's field surface, focus outline and placeholder color, with the id kept (see gaps). |
| Cursors | `widgets::apply_pointer_cursor` and `keep_arrow_cursor` are gone | Kit components set their own cursor: the hand on actions, the arrow on rows, tabs and menu items. |

### The cursor change you will notice

The old end-of-frame pass gave the pointing hand to *every* clickable plain egui widget. The kit
decides per component, so **plain `egui::Button`, `ui.button`, `ui.link`, `ui.checkbox` and
`selectable_label` now show the arrow** until their area is migrated to `Button`, `IconButton`,
`Link`, `Checkbox`, … (about 200 call sites; the tables below say where). This is expected, not a
bug to patch: do not bring the global pass back.

## Mapping: IDE widget or pattern to kit

| IDE today | Kit |
|---|---|
| `widgets::tree_row` / `TreeRow` | `components::ListRow` with `.depth`, `.expanded`, `.icon`, `.secondary`, `.label_tone`. Set `kit_expander: true` on `TreeRow` to get the kit's expander (the chevron becomes a button, `out.toggled` replaces the `response.clicked() && out.on_chevron()` test). For a long tree use `VirtualList` with `ListRow` (Tree if you do not need row behaviours). |
| `widgets::list_row` + `row_text` (caller paints) | `ListRow`; if the row is genuinely custom, `State` + `row_pill` + `RowContent` |
| `icons::button`, `button_colored`, hand-made toolbar buttons | `components::IconButton` (`.tone(Tone::Run)`, `.selected(..)`, `.small(true)` inside rows) |
| `egui::Button`, `ui.button`, `small_button` | `Button::primary / secondary / ghost / danger` |
| `ui.link`, hyperlink labels | `Link` |
| `ui.checkbox`, `radio_value`, toggles | `Checkbox`, `Radio`, `Toggle` |
| `selectable_label`, `selectable_value` rows | `SegmentedControl` (views), `ListRow` (lists), `TabBar` (tabs) |
| `egui::ComboBox` | `Dropdown` |
| `egui::TextEdit` in a frame, `widgets::search_field` | `TextField` / `TextField::search` |
| `egui::Frame::popup(..)`, hand-made popup areas | `Surface::Popup.frame(ctx)`, or `parts::Popover` for popups attached to a widget |
| Modal dialogs (`popups::popup_frame`) | `components::Dialog` (decisions, short forms) or `Surface::Dialog.frame(ctx)` for larger content |
| `island_frame`, tool-window title bars (`tool_window_header`) | `components::Panel` (title, actions), `Surface::Island` |
| Tooltips (`on_hover_text`) | `behaviour::Tooltip::new(label).shortcut(..).hint(..)` via `.with(..)` |
| `response.context_menu(..)` | `ContextMenu::new(\|ui\| { ui.add(MenuItem::new(..)); })` via `.with(..)` |
| Hand-made dragging | `DragSource(payload)` / `DropTarget::new(&mut dropped)` |
| Custom cursor code | `Cursor(icon)` behaviour, `cursor::override_in`, `cursor::fallback_in`; never `ctx.set_cursor_icon` in a panel |
| Banners (`disk_banner`, `gradle_banner`, `toolchain_banner`) | `Banner` |
| `notifications.rs` balloons | `Toast` |
| `busy_spinner`, `egui::Spinner`, `ProgressBar` | `Spinner`, `ProgressBar` |
| Empty states ("No file open") | `EmptyState` |
| Counts, tags, keyboard hints | `Counter`, `Tag`, `Kbd` |
| Property pages, key/value lists | `PropertyGrid`, `SectionHeader` |
| Editor tab strip | `TabBar` (with `DragSource`/`DropTarget` for tab moves) |
| Table-like lists (devices, keymap, plugins) | `Table` |
| `icons::Icon::X` | `Glyph::X` (see the table at the top of `icons.rs` for the renamed ones) or `Icon::for_file(name)` |
| Colors `theme::TEXT`, `MODIFIED`, … | `Tone::Default`, `Tone::Modified`, … or `Theme::colors(ctx).field` |
| Fonts `theme::semibold(size)` | `tokens::typography::semibold(ctx, size)` |

## Design lint

`cargo xtask design-lint` (run in CI) counts hand-rolled colors and shapes per file in
`jetthing-ui`, `jetthing-app`, `jetthing-plugin-ui` and `plugins/`: `Color32::from_rgb*`,
`0xRRGGBB[AA]` / `from_hex` palettes, `rect_filled` with a non-zero radius and
`CornerRadius::same`. Existing hits are grandfathered in `xtask/design-lint-baseline.txt`;
any count above its baseline fails. An area migration should bring its files to zero and run
`cargo xtask design-lint --update` so the baseline only ever goes down. Test modules are
not scanned.

## How to migrate an area

1. Pick an area from the list below and say so, so nobody else edits the same files.
2. Replace the area's painting and widgets with kit components, bottom up. Keep the
   commands, the state and the behaviour; change the pixels. Delete what you replaced.
3. Add a test with `archipel_kit::testing::audit` (next section) and, if the area has
   interaction worth keeping (context menus, drag, keyboard), a few `harness.get_by_label(..)`
   tests that exercise it the way a user does.
4. Run the screenshot script (below) before and after, and put the pictures in
   `docs/ui-migration/` with a short description in your PR.
5. Run the required checks from `AGENTS.md`. Do not commit `docs/parity/dashboard.md`.
6. If you hit something the kit cannot do, list it in your PR under "Kit gaps" and either send
   the kit a PR or use the documented workaround. Add it to the gap table below.

The reference migration is the **Structure** tool window and the **Project** tree
(`structure_view.rs`, `toolwindows.rs::project_tree`): `VirtualList` and `ListRow`, kit icons,
`ContextMenu` on every row, `DragSource` on files and `DropTarget` on folders (a drop is a cut
and a paste, like the Move dialog), the kit expander, and an audit test in `ui_audit.rs`.
Before and after screenshots, taken with real mouse input only, are in
[`docs/ui-migration/`](ui-migration/).

## Testing a migrated panel

`archipel-kit` is a dev-dependency of `jetthing-ui` with the `testing` feature. The audit
checks, in dark and light: every target has an accessible name, no target is smaller than
16×16, no two targets partly overlap, and Tab reaches everything.

```rust
#[test]
fn problems_tool_window_passes_the_audit() {
    for theme in [Theme::dark(), Theme::light()] {
        let app = app_with_project("problems");
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            panel(ui, |ui| crate::problems_ui::tool_window(app, ui));
        });
        harness.run();
        harness.get_by_label("Main.java"); // found by what a user sees
        testing::audit(&mut harness).assert_clean();
    }
}
```

Put it in `crates/jetthing-ui/src/ui_audit.rs`, which has the `app_with_project` and `panel`
helpers. Tips:

- Give the panel a fixed size (`panel` does); an unbounded panel makes virtualized lists
  lay out every row.
- Test with the real interaction: `.click()`, `.click_secondary()` and key presses on nodes
  found by label, then `harness.run()`.
- The audit presses Tab, so run it last.
- Snapshot tests render through wgpu. They need `apt-get install -y mesa-vulkan-drivers` on a
  headless machine; the audit tests do not.

### Screenshots under Xvfb

`docs/ui-migration/capture.sh <binary> <prefix>` starts the IDE on a demo project under Xvfb
(`:77`, 1440×900, `JETTHING_RENDERER=glow`) and drives it with xdotool mouse input only:
hover, right-click, expanding folders, opening a file, a drag and drop. Copy and extend it
for your area; run it once with the binary built from `main` and once with yours.

```bash
sudo apt-get install -y xvfb xdotool imagemagick libxkbcommon-x11-0 mesa-vulkan-drivers
cargo build -p jetthing-app
docs/ui-migration/capture.sh target/debug/jetthing-app /tmp/after
```

## Kit gaps found by the foundation

The foundation worked around these; each is a candidate for a kit PR. Fix the kit first when
your area needs one.

| Gap | Workaround today | Needed by |
|---|---|---|
| `TextField` has no `.id(..)`; tool windows move focus by id (`find_input_id`, settings search) | `widgets::search_field` draws the kit's field surface with the id kept | every `search_field` caller, Find, Settings |
| `DragSource` re-registers the row after its chevron, which breaks the Tab order of expandable rows (the audit reports every row "never focused by Tab") | Folders are not drag sources; files are | Project tree (folders), tabs |
| `Tree` takes no per-row behaviours; `VirtualList` drops Left/Right | `VirtualList` + `ListRow` + a small Left/Right handler in `structure_view.rs` | Structure, Project, Problems, Git log |
| `Dialog` has no close button, scrolling body, fit-to-window sizing, several buttons or a button row in Tab order; `TextField` has no way to select part of its text | `dialog::Shell` composes `egui::Modal`, `Surface::Dialog`, `Button` and `IconButton`; `dialog::focus_first` selects through the text edit's state | every dialog |
| `ListRow` has no two-line layout, leading avatar, ellipsis for long labels or tooltip; list rows hold only one `Slot` | Plugins list uses `Glyph::Plugin` and a `Slot::ui_width` with the button or checkbox; New Project widens the list so labels fit | Plugins, New Project |
| `ListRow` has no semibold label, arbitrary label or icon color, custom icon painter, leading checkbox slot, tinted background (excluded / test folders), or ellipsis for long text | `widgets::tree_row` falls back to kit parts for rows with a checkbox or custom icon, and ignores the font; tints are painted under the row; long text is clipped | Commit tool window, Git log, Gradle, build/test results |
| No submenu item in `MenuItem` | A `MenuItem` with a `›` plus egui's `SubMenu` (`context_menus.rs`) | menus, context menus |
| `IconButton` takes a `Tone`, not a color | `icons::button_colored` matches the color to the tone of the same value | AI chat, run widget, debug |
| `Surface` has no chrome variant (toolbar, status bar, stripes) | `chrome_frame(ctx)` fills with `colors.frame` | main toolbar, status bar |
| No tool-window title bar with a hide button | `tool_window_header` (icon buttons from the kit) | all tool windows (`Panel` may cover it) |
| No kit equivalent of `widgets::flat_widget` (status-bar and toolbar dropdown text with icon and chevron) | Uses `State` and `cursor::on_hover_action` | main toolbar, status bar |

## Areas

Estimates are for one worker who knows the area: **S** is up to half a day, **M** one to two
days, **L** three days or more. Areas that touch the same file (`chrome.rs`, `toolwindows.rs`,
`popups.rs`) should not be worked on in parallel; the notes say which.

| Area | Files | Est. | Notes |
|---|---|---|---|
| **Structure + Project tree** | `structure_view.rs`, `toolwindows.rs` (`project_tree`, `paint_virtual_row`), `context_menus.rs` | done | Reference migration. Left: the File Structure popup rows (`popups.rs`), call hierarchy (`hierarchy_ui.rs`, 112 lines, S). |
| Commit tool window | `toolwindows.rs` (`commit`, ~350 lines), `change_popup.rs`, `diff_ui.rs` (the diff viewer is separate: L) | M | Checkbox rows need the leading-slot gap. Same file as Find: coordinate. |
| Git tool window | `git_log_ui.rs`, `branches_popup.rs`, `push_dialog.rs`, `blame_gutter.rs` | L | Log is a `Table`; branches popup is a `Popover` with `ListRow`s (20 plain buttons). |
| Problems | `problems_ui.rs` | M | Grouped tree with counts: `ListRow` with `Counter` in the trailing slot. |
| Find (tool window, popup, find bar) | `toolwindows.rs` (`find`), `find_popup.rs`, `findbar.rs` | M | Needs the `TextField` id gap. Results are `ListRow`s with `Slot::text(count)`. |
| Run / Build / Gradle / Bazel / Test | `build_window.rs`, `test_view.rs`, `gradle_window.rs`, `bazel_window.rs`, `console_links.rs`, `build_variants_ui.rs` | L | Mostly tree rows with semibold group titles (gap) and run/stop `IconButton` tones. |
| Debug | `debug_window.rs`, `debug_dialogs.rs` | M | Variables as `Tree`; step buttons as `IconButton`s. |
| Terminal | `terminal_ui.rs`, `ai_terminal.rs` | S | Tab strip to `TabBar`; the terminal grid itself is not kit UI. |
| Logcat, devices, Android | `logcat_ui.rs`, `device_manager.rs`, `running_devices_ui.rs`, `android_view.rs`, `android_sdk.rs`, `merged_manifest_ui.rs`, `resource_manager_ui.rs`, `layout_editor_ui.rs` | L | Do after the generic tool windows. `device_manager` and `android_sdk` are also dialogs. |
| Editor tabs and chrome | `editor.rs` (`tabs`, `tab_drag`, ~450 lines), `completion_ui.rs`, `intentions_ui.rs`, `doc_ui.rs`, `folding.rs`, `blame_gutter.rs`, banners (`disk_banner.rs`, `gradle_banner.rs`, `toolchain_banner.rs`) | L | `TabBar` with `DragSource`/`DropTarget` for tab moves between splits. The text editor itself stays custom. |
| Popups and Search Everywhere | `popups.rs`, `palette.rs`, `navigation_ui.rs`, `switcher.rs`, `gutter_popup.rs`, `refactor_ui.rs`, `lsp_status.rs`, `notifications.rs` | L | 22 `Frame::popup` call sites to `Surface::Popup` / `Popover`. Do before the dialogs so they share the popup shell. |
| Dialogs | `new_project.rs`, `plugin_manager.rs`, `run_config_dialog.rs`, `clone_dialog.rs`, `push_dialog.rs`, `first_run.rs`, `about.rs`, `crash_notice.rs`, the `Dialog` enum in `popups.rs` | done | All on `dialog::Shell` (`dialog.rs`: backdrop, centring, fit-to-window, pinned button row, close button, Enter/Escape) and kit `TextField`, `Dropdown`, `Checkbox`, `PropertyGrid`, `VirtualList`. Before/after screenshots: [`ui-migration/dialogs/`](ui-migration/dialogs/). Left: `settings_dialog.rs` (+ `settings_keymap.rs`, `settings_live_templates.rs`, `settings_pages.rs`: L), Recent Files / Recent Locations in `popups.rs` (popups, with the popup stack work). |
| Main toolbar, status bar, stripes | `chrome.rs`, `main_menu.rs`, `toolchain_status.rs`, `lsp_status.rs`, `plugin_ui.rs` (status widgets) | M | One worker for all of `chrome.rs`. Run widget to `Button` + `Dropdown`; main menu needs the submenu gap. |
| Welcome screen | `welcome.rs` | S | `EmptyState`, `ListRow`, `TextField::search`, avatars (`Avatar`). |
| AI chat and plugin panels | `ai_chat.rs`, `ai_agent.rs`, `ai_actions.rs`, `ai_inline.rs`, `ai_vcs.rs`, `plugin_ui.rs`, `markdown_ui.rs` | M | Lowest priority; markdown rendering stays custom. |

### Order

1. Popups (they define the shared popup shell), then Dialogs.
2. Generic tool windows (Problems, Commit, Find, Run/Build, Git, Debug), in any order, one file owner at
   a time for `toolwindows.rs`.
3. Chrome, tabs, welcome.
4. Delete `icons::Icon` (replace with `Glyph` at the call sites), `widgets::tree_row`,
   `widgets::list_row` and the `theme::` color tokens once nothing uses them.
