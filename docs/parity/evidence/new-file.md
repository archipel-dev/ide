# Native file dialogs and New › Java Class

Reference: the Project tool window's **New** group (`NewGroup`: Java Class, Kotlin
Class/File, File, Directory/Package) and its small "New Java Class" popup (a name field
above a Class/Interface/Enum/Record/Annotation list), and the operating system's own file
chooser for File › Open.

## Behavior

- `project.newJavaClass {dir, name, kind}` (kind: `class|interface|enum|record|annotation`)
  and `project.newKotlinFile {dir, name, kind}` (`class|interface|enum|object|file`).
  `dir` is project-relative or absolute inside the project. The package comes from the
  directory below `src/<set>/java|kotlin` (`src/main/java`, `src/test/java`,
  `src/main/kotlin`); `util.Foo` adds sub-packages. Invalid names, unknown kinds, directories
  outside the project and existing files fail with a clear error. The new file opens with
  the caret inside the body.
- Open Project (welcome screen, File › Open…, Ctrl+O) and New Project › Browse… call the
  `FileDialogs` trait. `SystemDialogs` uses `rfd` (xdg-desktop-portal on Linux, NSOpenPanel,
  IFileDialog) on a background thread polled each frame. When there is no display or
  session bus the in-app path field is shown instead. `dialog.injectPath {path}` queues an
  answer for the next dialog (control socket, scripts); tests inject `FakeDialogs`.

## Reproduction (Linux, Xvfb)

```bash
Xvfb :99 -screen 0 1600x1000x24 &
export ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock DISPLAY=:99 ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1
cargo run -p archipel-ide-app -- /path/to/project &     # has src/main/java/com/example/Main.java
cli() { cargo run -q -p archipel-ide-cli -- ui "$@"; }
cli click 208 209 --button secondary; cli move 270 229   # New submenu
cli click 500 229; cli type model.Order; cli key Down; cli key Down; cli key Down
cli screenshot new-java-class-popup.png; cli key Enter; cli screenshot created-file.png
cli key Primary+O                                        # no portal on Xvfb: fallback
cli execute dialog.injectPath '{"path":"/path/to/other"}'; cli key Primary+O
```

![New submenu](new-file/new-submenu.png)
![New Java Class popup](new-file/new-java-class-popup.png)
![Created file](new-file/created-file.png)
![Open Project fallback](new-file/open-project-fallback.png)

`Order.java` was created at `src/main/java/com/example/model/Order.java` containing
`package com.example.model;` and `public record Order() {` with the caret on the
indented line in the body (4:5). Re-creating it reports `already exists`; `9x` reports
`not a valid class name`.

## Gaps

- The real OS dialogs (portal, NSOpenPanel, IFileDialog) were not exercised: headless Xvfb
  has no desktop portal, so CI covers the injected and fallback paths only.
- No Alt+Insert popup, no editable file templates, no Groovy/other language templates, the
  kind list has no icons.
