# 06. Structural Search and Replace (overview)

Status: specification overview, language independent. Requirement prefix `SRCH-SSR`.
Source basis: IntelliJ Community tag `idea/262.10968.63`. Paths: `ic: platform/structuralsearch/**`, `platform/structuralsearch/resources/messages/SSRBundle.properties`.
Priorities: `LATER` unless noted. Executable cases: `cases/06-structural-search.cases.md`.

Structural search finds code by syntax-tree shape instead of text. It is a large feature (its own dialog, template language, per-language profiles, inspections);
this file records the user-visible contract so that the text search and the later implementation stay compatible.

Actions (from the inventory): `StructuralSearchPlugin.StructuralSearchAction` (`Search Structurally…`) and `StructuralSearchPlugin.StructuralReplaceAction`
(`Replace Structurally…`), no default keys, in the Edit | Find menu group.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-SSR-001 | LATER | Both actions open a dialog with a **search template** editor (`Search template:`), a scope selector (as Find in Files, see `02-find-in-files.md`), predefined templates (`... search template` entries such as `Method references`, `Fields/variables read`), and a button to run. Matches go to the Find tool window (usage view of `03-find-usages.md` section 4), titled `fragments matching template '<template>'`; progress text `Found <N> matches`. | cases SSR-001/1 |
| SRCH-SSR-002 | LATER | **Template syntax:** the template is code of the file's language; identifiers between dollar signs (`$x$`) are **variables** matching any element of the allowed kind; a variable used twice must match equal code. Each variable can have *modifiers*: `Count` (min and max occurrences, `Count=<range>`), `Text` (regular expression on the matched text; `!` inverts), `Type` (`|`-separated type patterns; `!` inverts), `Reference` (the referenced element matches another template), `Script` (a scripting expression, disabled with `Scripts will not be executed because the project is not trusted` in untrusted projects), `Within type hierarchy`. | cases SSR-002/1..2 |
| SRCH-SSR-003 | LATER | **Replace:** a replacement template may use the variables; the preview dialog is titled `Structural Replace Preview`; replace happens as in `02` (confirmation, one undo). Errors: `Unknown search variable '<v>' or replacement variable '<v>' has no script`, `A non-expression cannot be replaced with an expression`, `Template recursively references itself`, `Constraints are only allowed on the first reference of a variable`, `Incorrect reg exp constraint: <re> for <v>`. | LATER cases |
| SRCH-SSR-004 | LATER | Structural search inspections (`Add Structural Search Inspection…`, `Add Structural Replace Inspection…`) and message `The specified template does not match anything in scope '<scope>'`. | n/a |
| SRCH-SSR-005 | LATER | Reason for LATER: it needs the language-neutral syntax tree matcher and a template language; not required for the first usable release. The text Find / Replace (`01`, `02`) must not depend on it. | n/a |
