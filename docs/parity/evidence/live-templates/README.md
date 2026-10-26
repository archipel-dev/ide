# Live templates and postfix completion: evidence

Scope: parity rows IJ-ED-130 (live template expansion), IJ-ED-131 (Insert Live Template, Surround with live
template) and IJ-ED-086 (postfix completion). Captured on Linux under Xvfb (software rendering) with the
`archipel-ide` debug build and no language server installed (`jdtls: not installed` in the status bar), so the
templates are the only completion source. Scenes were driven with the control socket
(`archipel-cli ui execute language.complete`, `key Tab`, `type k`, `key Ctrl+J`).

| File | Steps | Result |
|---|---|---|
| `01-sout-popup.png` | `Sout.java`, caret after `sout` inside a method, Ctrl+Space | The popup lists `sout`, `soutm`, `soutp`, `soutv` with the template icon and each description. |
| `02-sout-expanded.png` | Tab | `System.out.println();` with the caret between the parentheses. |
| `03-psvm-expanded.png` | `Psvm.java`, `psvm` in a class body, Tab | `public static void main(String[] args) { … }` indented like the line it replaced; caret in the body. |
| `04-fori-tab-stops.png` | `Fori.java`, `fori` in a method, Tab | `for (int i = 0; i < ; i++)`: the three `i` are selected as one mirrored variable. |
| `05-fori-mirrored-typing.png` | type `k` | All occurrences change together (3 carets in the status bar); Tab moves on to the limit. |
| `06-postfix-var-popup.png` | `Var.java`, `new ArrayList<String>().var`, Ctrl+Space | The postfix key `var` is offered after the expression. |
| `07-postfix-var-expanded.png` | Tab | `ArrayList<String> arrayList = new ArrayList<String>();` (type inferred from the `new` expression, name from its class). |
| `08-insert-live-template.png` | Ctrl+J in a method body | Popup of the templates that apply at the caret (statement context: no `psvm`). |
| `09-settings-live-templates.png` | Settings › Editor › Live Templates, `sout` selected | Groups with enable toggles, abbreviation / description / template text editor, applicable context. |

Mechanics covered by tests: `cargo test -p archipel-ide-templates` (parsing, `$VAR$`/`$END$`/`$SELECTION$`,
expressions, context detection, postfix rewriting, user templates) and `cargo test -p archipel-ide-core
live_templates` (popup items, Tab expansion, tab stops, `.var`, Insert / Surround commands, persistence).

## Still different from the reference

- Contexts come from a brace scanner, not a parser: Java statement / expression / declaration are told apart
  heuristically, Kotlin and Rust templates apply everywhere outside comments and strings.
- Postfix templates are not filtered by the expression's type (`.if` is also offered on a non-boolean);
  `.var` only knows types of literals and `new` expressions and otherwise falls back to `var`.
- Variable expressions are limited to `className()`, `methodName()`, `methodParameters()`,
  `suggestIndexName()`, `suggestVariableName()`, `expressionType()`; the settings page edits abbreviation,
  description and text, not the variable table.
- Surround With (Ctrl+Alt+T, language constructs) is a separate row (IJ-ED-124) and not implemented here;
  only the live-template flavour (Ctrl+Alt+J) is.
