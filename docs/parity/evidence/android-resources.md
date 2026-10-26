# Android resource navigation (AND-010)

Reference: Go to Declaration (⌘B / Ctrl+B) on `R.layout.activity_main` in `MainActivity.kt` and
on `@string/…` / `@style/…` attributes in XML, as in Android Studio (scenes a01/a07 show the
sources and manifest it navigates between).

Captured on Linux under Xvfb with the `MyApplication` sample (see `android-project-view.md`).

| File | Shows |
|---|---|
| `archipel-ide/android/a10-goto-layout.png` | caret on `R.layout.activity_main` in `MainActivity.kt`, Ctrl+B: `res/layout/activity_main.xml` opens, status `R.layout.activity_main` |
| `archipel-ide/android/a10-goto-style-qualifiers.png` | caret on `@style/Theme.MyApplication` in `AndroidManifest.xml`, Ctrl+B: `values/themes.xml` opens, status `R.style.Theme_MyApplication: 2 declarations` (`values` and `values-night`) |

## Behaviour

- References under the caret: `R.<type>.<name>` (also fully qualified `com.x.R.string.y`) and
  `@<type>/<name>` / `@+id/<name>`, with dots in XML names mapped as R does
  (`Theme.App` → `Theme_App`). Framework references (`android.R…`, `@android:…`) are not handled.
- The index is built from the project file index:
  - file resources (layout, drawable, mipmap, menu, xml, raw, anim, animator, color, font,
    navigation, transition, interpolator), named after the file without extensions (`x.9.png` → `x`);
  - value elements in `res/values*/` (string, array, plurals, color, dimen, bool, integer, style,
    `<item type="id">`, declare-styleable → styleable), each with its line;
  - `@+id/` declarations in layouts, menus and navigation graphs.
- The default-qualifier declaration opens first, with the caret on the declaring line. All
  declarations are returned by the command and counted in the status bar.

Commands: `android.resources.gotoDeclaration` (⌘B / Ctrl+B in Android projects),
`android.resources.find {type, name}`.

## Differences (not claimed)

- With several declarations there is no chooser popup: the default qualifier opens and the
  count is reported.
- No Ctrl+click, no Find Usages, no rename refactoring, no resource completion, and no
  framework (`android.R`) or library (AAR) resources.
- The index is rebuilt on each request (fine for typical projects; no persistent index yet).
