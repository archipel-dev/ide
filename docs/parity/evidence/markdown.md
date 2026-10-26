# Markdown editor: layout island, live preview, formatting and insertion

Linux, Xvfb 1600x1000, `ARCHIPEL_RENDERER=glow`, debug build, project with `README.md` and
`img/logo.png`. Reference: <https://www.jetbrains.com/help/idea/markdown.html>. Parity rows:
`IJ-ED-151`, `IJ-ED-157..160` in `docs/parity/intellij-editor.md`.

| File | Shows |
|---|---|
| `markdown/split-dark.png`, `markdown/split-light.png` | Split layout in Islands Dark / Islands Light: editor with tree-sitter highlighting on the left; preview on the right with headings, emphasis, task list, ordered list, blockquote, relative image, highlighted Rust code block and an aligned table. Editor / Split / Preview island top-right |
| `markdown/floating-bar-dark.png` | Selecting text shows the floating bar: Bold, Italic, Strikethrough, Code, Link, H1-H3, bullet list |
| `markdown/preview-only-dark.png` | Preview layout after clicking the island's "Preview" |

Scroll sync was driven with `xdotool click 5` over the preview: the editor followed to the same
section (and the reverse for editor scrolling). Every action is a `markdown.*` command
(`markdown.layout`, `.bold`, `.italic`, `.strikethrough`, `.code`, `.link`, `.heading`, `.list`,
`.insertTable`, `.insertImage`, `.insertCodeBlock`, `.insertToc`, `.exportHtml`); keymap bindings
exist for the actions the reference keymap defines (`Markdown.Styling.CreateLink`,
`Markdown.InsertEmptyTable` from menus).

Automated coverage: `archipel-ide-markdown` (parser to block model, formatting transforms on
selections including Unicode, scroll mapping), `archipel-ide-core::markdown_commands` (one undo step per
edit, per-file layout, HTML export), `archipel-ide-ui::markdown_ui` (scroll sync feedback guards).

Known gaps: links in the preview are styled but not clickable; no math, Mermaid, table inlay
editing, Word import or PDF export; preview scroll mapping is block-granular.
