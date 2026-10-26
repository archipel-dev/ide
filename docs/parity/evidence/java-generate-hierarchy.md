# Generate, Super Method, Go to Test and hierarchies: evidence

Reference: IntelliJ IDEA *Generate* (Alt+Insert / ⌘N), *Go to Super Method* (Ctrl+U / ⌘U), *Go to Test*
(Ctrl+Shift+T / ⇧⌘T), *Type Hierarchy* (Ctrl+H) and *Call Hierarchy* (Ctrl+Alt+H); spec rows `JAVA-GEN-*`,
`JAVA-NAV-011/012/016/018` in `docs/spec/features/java-gradle.md`.

## Environment

No jdtls is installed in the capture environment, so the scenes run against the scriptable
`archipel-fake-lsp` configured as the `jdtls` server (`.archipel/lsp.toml`: `[servers.jdtls]
command = ".../archipel-fake-lsp"`) over a three-file Java project (`Shape`, `Base implements Shape`,
`Circle extends Base`). The fake answers from word-level parsing of `class … extends/implements` and
`void name(` lines, so the `detail` column shows its `fake` marker. Real jdtls answers are not captured yet.
Xvfb 1440×900, `ARCHIPEL_CONTROL_SOCKET` driving (`docs/parity/evidence/ui-control-socket.md`).

## Scenes

| File | Steps | Result |
|---|---|---|
| `generate-popup.png` | caret in `Base.java`, Alt+Insert | Popup with Constructor, Getter, Setter, Getter and Setter, equals() and hashCode(), toString(), Override Methods…; entries the server does not offer are greyed out with a reason. |
| `type-hierarchy.png` | caret on `Base`, Ctrl+H | Hierarchy tool window "Subtypes of Base": root `Base`, child `Circle`. |
| `type-hierarchy-supertypes.png` | "Show Supertypes" | "Supertypes of Base": `Shape`. |
| `call-hierarchy.png` | caret on `helper`, Ctrl+Alt+H | "Callers of helper": `Circle.area`; "Show Callees" flips the direction. |
| `go-to-super-method.png` | caret on `Circle.area`, Ctrl+U | Opens `Base.area` (a second Ctrl+U reaches `Shape.area`). |

Mechanics covered by tests: `cargo test -p archipel-ide-core --test generate_hierarchy` (needs the fake server
built: `cargo build -p archipel-ide-lsp --bins`) and `cargo test -p archipel-ide-core test_nav`.

## Real mouse and keyboard pass (xdotool, Xvfb 1280×800)

Driven only with `xdotool` (`windowfocus`, `mousemove`, `click`, `key`), no command dispatch; screenshots are
`mouse-*.png`: `mouse-generate-hover.png` (Alt+Insert popup, hover highlight on "Getter and Setter", inside the
window), `mouse-generate-click-getter.png` (a click on an entry applied it: status "Applied: Generate Setters"),
`mouse-type-hierarchy-hover.png`, `mouse-hierarchy-button-hover.png`, `mouse-type-hierarchy-supertypes.png`
(clicking "Show Supertypes"), `mouse-call-hierarchy.png` (Ctrl+Alt+H) and `mouse-super-method.png` (Ctrl+U).
Observed: Alt+Insert, Ctrl+H, Ctrl+Alt+H and Ctrl+U work from real key events; the Generate popup (the intention
popup) has hover states, Up/Down/Enter/Escape and stays in the window. Known gaps: the Hierarchy window has no
keyboard navigation yet (arrows, Enter, Escape) and no speed search; no dialog was added, so default-button and
Tab-order checks do not apply.

## Still different from the reference

- Generate entries come from the server's `source.generate.*` code actions; jdtls actions that ask the client
  for choices (fields, members) run as the server's own commands without an Archipel IDE dialog yet.
- Go to Super Method searches the supertypes' text for the declaration (the server has no super-method
  request); overloads are not told apart by signature.
- The hierarchy has one tree per direction; scope selector, "base on this class" and Method Hierarchy
  (Ctrl+Shift+H) are not implemented.
- Go to Test uses source-set and name conventions (`FooTest`, `FooTests`, `TestFoo`, `FooIT`), and Create Test
  writes an empty class without the "Create Test" dialog (library, methods, setUp/tearDown).
