# UI-COL: colors

Reference theme "Dark" (New UI). Normative values are the theme/scheme files (`ic: platform/platform-resources/src/themes/expUI/expUI_dark.theme.json`, `expUI_darkScheme.xml`); "m" is the pixel measured in the reference capture. Archipel column: New UI Dark / new_ui capture; the settings default (Islands Dark) is listed at the end.

## Requirements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-COL-001 | MUST | The default theme and look of a fresh install is the reference's: New UI Dark with the new_ui look (no islands, no 4 px gaps). Archipel's default is `Islands Dark` / `islands`. | Fresh config: frame/toolbar `#2B2D30`, editor `#1E1F22`. |
| UI-COL-002 | MUST | Every color in the table equals the theme value within 2 per channel in the matching Archipel surface. | Pixel probes per row. |
| UI-COL-003 | MUST | Colors come from the palette Gray1..Gray14 (`#1E1F22 #2B2D30 #393B40 #43454A #4E5157 #5A5D63 #6F737A #868A91 #9DA0A8 #B4B8BF #CED0D6 #DFE1E5 #F0F1F2 #FFFFFF`), Blue6 `#3574F0`, Green `#5FAD65`..., Yellow1 `#3D3223`, Green2 `#273828`. | - |

## Values

| Surface | Theme/scheme | m (IntelliJ) | Archipel New UI Dark | Diff |
|---|---|---|---|---|
| Toolbar, stripes, tool window, status bar | Gray2 `#2B2D30` | `#2B2D30` | `#2B2D30` | 0 |
| Editor, gutter, tab strip | Gray1 `#1E1F22` | `#1E1F22` | editor `#1E1F22`; tab strip `#2B2D30` | tab strip wrong |
| Border/divider | Gray1 `#1E1F22` | `#1E1F22` | `#1E1F22` | 0 |
| Under-tabs border | Gray3 `#393B40` | `#393B40` | none | missing |
| Popup border | Gray4 `#43454A` | `#434549` | `#43454A` | 0 |
| Popup header/footer band | Gray3 | `#393B40` | `#2B2D30` | differs |
| List/tree selection (focused) | Blue2 `#2E436E` | `#2D426D`/`#2D436D` | `#2E436E` | 0 |
| Completion selection | Gray4 `#43454A` | `#43454A` | blue (old capture `#113A5C`) | differs |
| Caret row | `#26282E` | `#25282D` | `#26282E` | 0 |
| Line numbers / current | `#4B5059` / `#A1A3AB` | `#484C54` / `#9DA1A8` | `#5D5F66` / `#A1A3AB` | non-current too light |
| Active stripe button, primary button, focused underline | Blue6 `#3574F0` | `#3574F0` / `#3474F0` | `#3574F0` | 0 |
| Unfocused underline | Gray6 `#5A5D63` | `#5A5D62` | n/a | - |
| Excluded folder row | Yellow1 `#3D3223` | `#3D3223` | none | missing |
| Test sources row | Green2 `#273828` | `#273828` | none | missing |
| Identifier under caret | `#373B39` | `#363A39` | not seen | - |
| Folded text bg | `#393B40` | `#393B40` | `#393B40`-like | 0 |
| Search match in lists | Yellow5 `#BA9752` | `#B59252..#BA9752` | blue / teal `#114957` | differs |
| Added / modified gutter bar | `#549159` / `#375FAD` | `#538F59` | not seen | - |
| Keyword / string / number / comment | `#CF8E6D` `#6AAB73` `#2AACB8` `#7A7E85` | LCD-fringed, consistent | not measurable (grayscale AA); same hues visually | - |
| Status text | Gray9 `#9DA0A8` | - | - | not measured |
| Hover fills | `#FFFFFF16` (action), Gray3 (list) | not captured | - | live |

Settings-default Islands Dark (fresh config capture): frame `#1A1B1E`, island panels and editor `#111213`, popups `#26282C`, 4 px gaps between islands, selected stripe button `#264E9A`; none of these exist in the IntelliJ 2025.2 Dark reference.
