# Editor multi-caret, column selection and line transforms

Linux, Xvfb 1600x1000, `ARCHIPEL_RENDERER=glow`, debug build, sample Rust file
(`fn main` using `value` four times plus a `NAMES` array of strings). Input was driven with
`xdotool` after `xdotool windowfocus`. No IntelliJ reference capture exists for these scenes
yet; the reference behaviour is the `PLATFORM-EDIT-*` rows in `docs/spec/features/platform.md`.

| File | Steps | Shows |
|---|---|---|
| `editor-multicaret/linux-next-occurrence.png` | click inside `value` on line 2, `alt+j` ×4 | the word, then each further whole-word `value` (including the one inside the format string); status bar `4 carets`; no `j` typed by the Alt chord |
| `editor-multicaret/linux-multicaret-typing.png` | type `amount` | all four occurrences replaced; one `ctrl+z` restores all of them (single undo step) |
| `editor-multicaret/linux-column-block.png` | Alt+drag from line 12 to line 15 inside the strings, `alt+shift+Insert`, `shift+Right` | rectangular selection, one caret per line; status bar `Column` and `4 carets` |

Automated coverage: `archipel-ide-text` (`multicaret`, `transform`, `ops`, `SelectionSet` typing
coalescing), `archipel-ide-core` (`multicaret_and_transform_actions_run_through_the_registry`),
`archipel-ide-ui` (`editor_keys` keymap and status label tests).
