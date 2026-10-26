# Notice for derived data

The JSON files in this directory and the generated Markdown files one level up
(`keymaps.md`, `menus.md`, `settings-tree.md`, `toolwindows.md`, `themes.md`) are derived
from declarative data files in **IntelliJ Community**:

- Repository: https://github.com/JetBrains/intellij-community
- Tag: `idea/262.10968.63` (IntelliJ IDEA 2026.2.x)
- License: Apache License 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
- Upstream NOTICE: "This software includes code from IntelliJ IDEA. Copyright (C) JetBrains s.r.o."

Only facts were extracted: action ids and labels, menu/group structure, default keyboard and
mouse shortcuts, settings page ids and names, tool window registrations, Advanced Settings
defaults, theme color tokens and editor color scheme values. No Java or Kotlin source code,
icons, images or other artwork were copied. The extraction is reproducible with
[`../tools/extract_intellij_data.py`](../tools/extract_intellij_data.py).

IntelliJ, IntelliJ IDEA and JetBrains are trademarks of JetBrains s.r.o. Archipel IDE is not
affiliated with or endorsed by JetBrains.
