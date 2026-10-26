# Workbench bug fixes (#47)

Fixes for bugs found in the Android Studio / IntelliJ comparison walkthrough
(`docs/parity/evidence/archipel-ide/NOTES.md`, "Bugs found"). Each fix has a regression test.
Captured 2026-10-10 on Linux/Xvfb (glow) with `xdotool`.

## 1. Missing desktop library at startup

Before: `thread 'main' panicked at xkbcommon-dl-0.4.2/src/x11.rs:59:28: Library
libxkbcommon-x11.so could not be loaded.` plus a backtrace.

After (library moved aside, `DISPLAY=:99 archipel-ide`):

```text
archipel-ide: cannot start: the system library libxkbcommon-x11.so is missing.
Install it with your package manager and start Archipel IDE again.
  Debian/Ubuntu: sudo apt install libxkbcommon-x11-0
  Fedora:        sudo dnf install libxkbcommon-x11
```

Exit code 1. Test: `missing_xkbcommon_gets_install_instructions` (archipel-ide).

## 4. Unbound Alt+digit typed the digit

Pressing Alt+7 in the editor no longer inserts "7" (the line in
`workbench-bugs/code-menu-alt7-badge.png` ends in the typed `x` only). Text that
AltGr / ⌥ compose into a different character is still typed. Test:
`unbound_alt_digit_does_not_type` (keys.rs).

## 6. Empty Code menu

The menu looked up a category that no command uses. Code now lists Comment with Line
Comment, Move Line Down/Up and Code Completion, like the reference's Code menu. Those
editor actions moved out of Edit. Test: `every_menu_has_items_and_code_lists_code_actions`.

![Code menu, badge on the modified tab, Alt+7 not typed](workbench-bugs/code-menu-alt7-badge.png)

## 7. Save and Save All both showed Ctrl+S

Per `docs/spec/keymaps.md`, Save All is ⌘S / Ctrl+S and Save has no default binding.
Ctrl+S already ran Save All. Test: `menu_shortcuts_are_unique_on_every_platform`.

![Edit menu: Save without a shortcut, Save All Ctrl+S](workbench-bugs/edit-menu-save-all.png)

## 15. Small issues

- "1 changes on master" now reads "1 change on master" (`change_summary_uses_singular_for_one_change`).
- Long names in the project tree end in "…" at the island edge and show the full name in
  a tooltip (`long_names_are_elided_with_an_ellipsis`).
- The modified badge moved from over the end of the tab title to the file icon's corner
  (`modified_badge_does_not_cover_the_tab_title`).

![Elided tree name with tooltip](workbench-bugs/tree-ellipsis-tooltip.png)
