# Parity inventory format

Every Markdown file in this directory except `README.md` and the generated
`dashboard.md` is an **area**. `cargo run -p xtask -- parity` parses all areas and
regenerates `dashboard.md` and the summary block in the top-level `README.md`;
`cargo run -p xtask -- parity --check` fails when any area is malformed or an ID is
reused. Feature PRs should not commit the generated files, so parallel PRs that add rows
do not conflict; they are regenerated on main. `cargo test -p xtask` also validates the committed inventory.

## Tables

A parity table is a Markdown table whose header row names at least an `ID` and a
`Status` column. Columns are found by header name, so their order is free. The
canonical 8-column header is:

```markdown
| ID | Reference | Feature | Status | Weight | Commands | Tests | Limitations / evidence |
|---|---|---|---|---:|---|---|---|
```

The older 7-column header (no `Weight`) is still accepted, and every row in such a
table has weight 1. Tables whose header lacks `ID` or `Status` are ignored, so areas
may contain other tables.

Rules enforced by the parser (errors point at `file:line`):

- every data row has the same number of cells as its header (escape a literal pipe as `\|`);
- `ID` is non-empty and unique across **all** areas — prefix IDs by area, for example
  `IJ-ED-001` or `PLAT-001`;
- `Status` is exactly one of `Missing`, `Planned`, `Partial`, `Implemented`, `Verified`;
- `Weight` is empty (meaning 1) or an integer from 1 to 100. The IntelliJ catalog uses
  1-5: 5 = core daily workflow, 3 = significant, 2 = moderate, 1 = minor.

Put `[Ultimate]` in any cell (conventionally at the start of `Feature`) to mark an
IntelliJ IDEA Ultimate-only feature; the dashboard reports these separately.

## Status meanings

- **Missing**: nothing exists in Archipel IDE yet.
- **Planned**: a design or command name exists, no working behavior.
- **Partial**: some of the listed scope works; `Limitations / evidence` says what is missing.
- **Implemented**: the listed scope works and the `Tests` column names the evidence.
- **Verified**: independently checked against the reference, with reproducible evidence
  recorded in this directory. Do not use it without that evidence.

## Scoring

`% implemented` = (weight of Implemented + Verified rows + half the weight of Partial
rows) / total weight. `% verified` = weight of Verified rows / total weight. Both are
reported overall, per area, and per edition scope.
