# Find in Files options (IJ-NAV-064, IJ-NAV-065)

Reference: the Find in Files popup has Match Case (Alt+C), Words (Alt+O) and Regex
(Alt+X) toggles and a File mask filter. Changing an option re-runs the search.

Archipel IDE (captured 2026-10-10 on Xvfb/glow; query `greet` in a sample with `Greeter.java`
and `GreeterTest.java`):

![Plain search, Words on, then the file mask !*Test.java](workbench-bugs/find-in-files-options.png)

1. Default: case-insensitive substring, 5 lines with every occurrence highlighted.
2. **W** (Words) on: only `greet` as a whole word, so 2 lines.
3. File mask `!*Test.java`: test sources excluded, so 1 line.

- `search.project {query, matchCase?, wholeWords?, regex?, fileMask?}`. Toggling an option
  or pressing Enter in the mask field re-runs the search.
- Matching uses the `regex` crate in every mode: a plain query is escaped, Words wraps it
  in `\b…\b`, and case folding is Unicode-aware. Results report character columns, so
  multibyte text highlights correctly. Empty regex matches never produce rows.
- File mask: globs separated by commas or semicolons (`globset`). `!` excludes. A pattern
  without `/` matches the file name, otherwise the project-relative path
  (`*.java, *.kt`, `!*Test.java`, `src/test/**`).
- An invalid regex or mask is rejected as invalid parameters and the previous results stay.

Tests: `match_case_whole_words_and_regex_options`, `file_masks_include_and_exclude`,
`multibyte_text_maps_to_character_columns` (search),
`project_search_honours_options_and_rejects_bad_regex` (core),
`find_options_become_command_parameters` (UI).

Gaps: no scopes (module, directory, custom scope), no Alt+C/O/X shortcuts, no preview pane,
and no Replace in Files.
