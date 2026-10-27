# Code insight specification (clean-room)

Requirements for the Archipel IDE code-insight stack, written from observed behaviour and public
API names of IntelliJ IDEA Community (source basis: tag `idea/262.10968.63`; cited as `ic: ...`).
No code, comments or structure are copied. Each requirement has a stable ID, a one-line
acceptance criterion and a priority. Scenarios (`S-xxx-n`) give concrete input -> output flows.

## Priority legend

- MUST: needed for the first usable Java/Kotlin/Spring release.
- SHOULD: expected for parity, scheduled after the MUSTs.
- LATER: specified for completeness, not scheduled.

## Files

| File | Topic | ID prefix |
|---|---|---|
| 01-daemon.md | Highlighting daemon: passes, restarts, visible range, cancellation, status widget | CI-DMN |
| 02-highlight-info.md | HighlightInfo model, severities, attributes keys, stripe, tooltips | CI-HLI |
| 03-inspections.md | Inspections, profiles, suppression, batch run | CI-INS |
| 04-intentions-and-alt-enter.md | Quick fixes, intentions, ModCommand, Alt+Enter assembly | CI-FIX, CI-ALT |
| 05-auto-import.md | Auto-import hint, settings, import fixes | CI-IMP |
| 06-completion.md | Completion framework | CI-CMP |
| 07-navigation.md | Navigation, usages | CI-NAV |
| 08-optimize-imports-and-reformat.md | Optimize imports, reformat and editing hooks | CI-OPT |

## Requirement counts (by priority)

| Priority | Count |
|---|---|
| MUST | 206 |
| SHOULD | 68 |
| LATER | 7 |

Per file (MUST/SHOULD/LATER): 01 46/12/0; 02 25/4/0; 03 28/9/4; 04 34/13/2; 05 18/4/0;
06 32/8/0; 07 11/12/1; 08 12/6/0. (Counted from the requirement lines; rerun the grep in
`CI-... MUST|SHOULD|LATER` form to refresh.)

## Disagreements with existing docs

Listed in detail in `01-daemon.md` section 8. Main ones: passes use two dependency relations;
visible-range-first is inside a pass; the 300 ms delay applies only after a cancelled pass;
identifier highlighting is not a pass; the trailing suppress comment is not an IntelliJ form.

## Open questions

- Verify shipped defaults of `Add unambiguous imports on the fly` (source: off) and intention
  preview (source: on) in a running IDE.
- Kotlin specifics beyond importer behaviour and settings.
- Default shortcuts of a few actions; intention-preview toggle shortcut on non-macOS.
- Whether F3 steps through highlighted usages; Find Usages excess-results threshold.
- Feature-usage advertisement threshold (3 uses is an Archipel proposal); statistics store bound.
- The matcher golden table (CI-CMP) must be generated from the reference matcher by the
  implementation team.
