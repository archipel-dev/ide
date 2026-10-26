# Tool window consistency (layout tokens, shared rows)

Captured under Xvfb with the glow renderer. `before-*` is `main` at `b4836d0` + #215/#216,
`after-*` is this branch. Each shot toggles the tool window through the control socket
(`toolwindow.toggle`) over a throw-away git project with uncommitted changes; sizes are
1440x900 and 1280x800 (`<label>-<window>-<size>.png`; windows: structure, commit, project,
git, find, problems, terminal, build, gradle).

`manual-*` shots use real X11 pointer input (`xdotool mousemove`/`click`): hover and select
a Structure row (navigates the editor), switch to Project, hover and click a tree row.

What the shots show:

- Structure rows use the Project row pill (x 46..314 at 1280 wide) and the same chevron
  column; items with children (e.g. `tests`) get a working expander.
- Git Log: the filter toolbar wraps and every pane clips to its rect, so the `Date`
  dropdown and the commit table no longer paint over the details panel (`7 files`).
- Commit: toolbar glyphs, `Amend` and the `Changes` header share the 12 px content inset.
- Empty states use `icon_label`, centring the icon on the text's line box.

Pixel checks are unit tests in `crates/archipel-ide-ui/src/widgets.rs`
(`icon_label_centres_the_icon_on_the_text_line_box`,
`tree_rows_share_one_inset_and_icon_column`).

Not covered: Bookmarks, TODO and Notifications tool windows do not exist as docked windows
in this tree (Notifications are toasts in `notifications.rs`, which only picked up the
dialog padding token); Run/Build trees were captured empty (no task running).
