# 07. Name matching and ranking (camel humps, wildcards, scores)

Status: specification, language independent. Requirement prefix `SRCH-MAT`.
Source basis: IntelliJ Community tag `idea/262.10968.63`. Paths are cited as `ic: ...`.
Priorities: `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.

This is the shared matcher behind Search Everywhere, Go to Class / File / Symbol / Action,
the Recent Files speed search, the find-bar completion popup and (with other options) code
completion. Every ranked list in `04-search-everywhere.md` and `05-recent-and-structure-popups.md`
is the result of this file plus the list rules there. Executable cases: `cases/07-matching-and-ranking.cases.md`.

ic: `platform/util/text-matching/src/com/intellij/psi/codeStyle/MinusculeMatcherImpl.kt`,
`.../MinusculeMatcher.kt`, `.../NameUtil.java`, `.../PreferStartMatchMatcherWrapper.kt`,
`.../WordPrefixMatcher.kt`, `.../FixingLayoutMatcher.kt`, `platform/util/base/src/com/intellij/util/text/NameUtilCore.kt`,
`platform/util/text-matching/src/com/intellij/util/text/matching/MatchingMode.kt`,
`platform/lang-impl/src/com/intellij/ide/util/gotoByName/DefaultChooseByNameItemProvider.java`,
`.../ide/actions/GotoFileItemProvider.java`, `.../ide/util/gotoByName/ChooseByNamePopup.java`.

## 1. Model

A **matcher** is built from a *pattern* (what the user typed), a *matching mode* and an optional
set of *hard separator* characters. For one candidate *name* it answers:

1. `match(name)`: no match, or an ordered list of **fragments** (half-open ranges `[start,end)` of
   the name, ascending, non-overlapping, never adjacent: adjacent ranges are merged). An empty
   pattern matches every name with zero fragments.
2. `degree(name, fragments)`: an integer score. Higher is better. No match scores the minimum integer; zero fragments
   score `0`.

The fragments drive the highlighting in result lists (bold or colored ranges). The score drives order.

**Matching modes** (`ic: MatchingMode.kt`):

| Mode | Case rule |
|---|---|
| `IGNORE_CASE` (default for all navigation) | a pattern letter matches a name letter of either case; case only influences the *score* and whether an uppercase pattern letter may jump to a lowercase name letter (rule 3.4) |
| `FIRST_LETTER` | as `IGNORE_CASE`, but the first non-wildcard pattern letter (skip leading `*` or space) must have the same case as the very first letter of the name, when it has a case |
| `MATCH_CASE` | letters must match exactly |

## 2. Pattern preparation

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-MAT-001 | MUST | The wildcards are `*` and the space character. A wildcard matches any run of name characters, including none. A trailing `* ` (star, then space) is dropped before matching. Wildcards never need to appear in the name. | cases 07.1 to 07.4 |
| SRCH-MAT-002 | MUST | *Word separators* in the pattern are whitespace, `_`, `-`, `:`, `+`, `.`. The pattern is **mixed case** when it contains a lowercase letter and an uppercase letter that has at least one non-wildcard character before it (so `*Foo` is not mixed, `fooBar` is). Mixed case forbids a lowercase pattern letter from jumping to a later word start (rule 3.5). | case 07.9 |
| SRCH-MAT-003 | MUST | Quick reject: the name must contain every non-wildcard pattern character in order, each in either case, and the name must not be shorter than the number of such characters. | case 07.5 |
| SRCH-MAT-004 | SHOULD | A pattern longer than 100 characters switches to plain matching without humps: without a leading `*` the name must start with the pattern's non-wildcard characters (wildcards in the name are skipped); with a leading `*` the first position where that holds anywhere in the name is used. Result: one fragment. | unit test with a 101-character pattern |
| SRCH-MAT-005 | SHOULD | A matcher may be built with a **fallback pattern** (for example the pattern re-typed on the other keyboard layout, Cyrillic to Latin). A name matches when either pattern matches; the first pattern that matches provides fragments and score. | pattern `ыекштп` finds `String` on a Russian-to-English layout |

## 3. Matching rules

A **word start** (`isWordStart(name, i)`) is true at index `i` when: the char is an uppercase letter that is not inside an
all-capitals run (an uppercase letter preceded by an uppercase letter is a word start only if the next char is lowercase),
or the char is a digit, or it is a letter at index 0 or preceded by a non-letter-or-digit, or it is an ideographic character
(each is its own word). A non-letter, non-digit char is never a word start.

A **hump** (camel-hump segment) is the next-word segmentation used for scoring: a digit run is one hump per digit; a
run of capitals followed by a lowercase letter ends one char early (`URLConnection` = `URL`, `Connection`); otherwise
a capital plus following lowercase letters; a non-alphanumeric char is its own hump.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-MAT-010 | MUST | The pattern is cut into **fragments** that each match a contiguous run of name characters, in order. A fragment is the longest run starting at a candidate name position where pattern and name chars are equal (case rule of the mode). Between fragments the name may have any characters, subject to rules 3.3 to 3.6. | cases 07.5 to 07.12 |
| SRCH-MAT-011 | MUST | **Start anchoring.** Without a leading wildcard, the first fragment must start at name index 0. With a leading `*` or space the first fragment may start anywhere subject to 3.6. | cases 07.8, 07.30 |
| SRCH-MAT-012 | MUST | **Skipping**: after a fragment, the next fragment may begin at a later *word start* of the name (not in the middle of a word) when the next pattern char is a letter or digit and the previous pattern char is not `*`. If the previous pattern char is `*` or the next pattern char is a word separator (`_ - : + . ` and whitespace), the next fragment may begin at any position (`ic: indexOfWordStart`). | cases 07.6, 07.7 |
| SRCH-MAT-013 | MUST | **Uppercase pattern letters.** When the next fragment start is searched along word starts (rule 012), an uppercase pattern letter can only land on a word start (`HM` does not match `Hashmap`). When the position comes from the `*`/separator route (any position), an uppercase pattern letter may land on a lowercase letter that is not a word start only if the pattern is not mixed case and the mode is not `MATCH_CASE` (`*M` matches `hashmap`, `*aM` does not). Inside a fragment, letters compare ignoring case in the two case-insensitive modes. | cases 07.2, 07.14, 07.15 |
| SRCH-MAT-014 | MUST | **Lowercase pattern letters in a mixed-case pattern** cannot start a later fragment by skipping to a word start, unless the previous pattern char is `*` or a word separator. They can still extend the current fragment. (`hamap` matches `HashMap` by skipping to `Map` because the pattern is all lowercase; the capital first letter of `Hamap` does not make a pattern mixed, `haMap` and `hM` do.) | case 07.9 |
| SRCH-MAT-015 | MUST | **Hard separators.** When a set of hard separator chars is configured (code completion passes space and parentheses; navigation passes none) and the pattern has no separators and is not mixed case, a fragment may not skip over a hard separator. | unit test |
| SRCH-MAT-016 | MUST | **Dots.** If the pattern contains a dot, a skip between fragments may not cross a name dot unless the preceding pattern char is itself a dot; one pattern dot may match several name dots at the fragment. (`foo.goo` matches `foo.bar.goo`, `*.ico` does not match `sm.th.iks.concierge`.) | cases 07.13, 07.15 |
| SRCH-MAT-017 | MUST | **Digits.** Each name digit is its own hump. A pattern digit that directly follows another pattern digit must not be matched against a *different digit* that follows the previously matched digit in the name: `12` does not match `13`, `a12` does not match `a13`. It does match `12` and `1x2` (the second digit is found at the next word start, because digits are word starts), and `a12` matches `a1b2`. | cases 07.11, 07.16 |
| SRCH-MAT-018 | MUST | **Middle matches.** A fragment that starts after a `*` inside a word (not at a word start), while more pattern follows it, may be shortened to let later fragments match only down to three characters; a shorter fragment fails. If the fragment already covers the remaining pattern, any length is fine. So `*ap` matches `Map`, but `*am` does not match `hashmap` (the lone `a` would have to be followed by `m`) and `*aeb` does not match `MapEbc` while `*apeb` does. | cases 07.17, 07.18 |
| SRCH-MAT-019 | MUST | **Hump improvement.** When a prefix fragment is longer than the uppercase letters of the pattern need (`CU` against `CurrentUser`: the prefix `Cu` matches), the matcher first tries to re-anchor each uppercase pattern letter on a later word start (`C` + `U`), and uses the improved split when the remainder matches. Otherwise it backs off the fragment length one char at a time and tries the rest. | case 07.12 |
| SRCH-MAT-020 | SHOULD | **Trailing space.** A pattern ending in a single space whose last real char is not an uppercase letter or a digit requires a literal space in the name after the last fragment, unless the last fragment ends at the end of the name. The space is highlighted. `foo ` matches `foo bar` and `foo`, not `foobar`; `fb ` does not match `foo bar`. A last real char that is uppercase or a digit adds no such requirement. | case 07.19 |

## 4. Score (matching degree)

The degree of a successful match is the sum of the terms below. All constants are normative; they decide ties between
candidates and are visible in the order of every result list.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-MAT-030 | MUST | `+1000` when the first fragment starts at a *whole word start*: index 0, or a word start whose predecessor is not a word start (the first hump of a word, not a capital in the middle of an all-capitals run). | cases 07.20 to 07.23 |
| SRCH-MAT-031 | MUST | `-1` per fragment, and `-10` per name hump that lies **between** the first fragment and a later fragment and is skipped completely (a gap in the camel humps). | case 07.24 |
| SRCH-MAT-032 | MUST | `+2` when no hard-separator char precedes the first fragment (always true when no hard separators are configured), `+1` when the first fragment starts at 0 (start match), `+1` when the last fragment ends at the end of the name. | cases 07.20 to 07.23 |
| SRCH-MAT-033 | MUST | **Case term**, summed per matched name character, using the pattern char it consumed: an uppercase pattern char equal to the name char `+50`; (only when the caller asks for "valued start case match", which completion does and navigation does not) the first name char equal to the first pattern char `+150`; a lowercase pattern char equal to a lowercase hump start `+1`; a hump start whose case differs from the pattern char `-1`; a lowercase pattern char matched against a lowercase char that follows an uppercase hump start the pattern matched as uppercase `-1`; a lowercase pattern char that starts a hump after a gap `-10`. | cases 07.20 to 07.26 |
| SRCH-MAT-034 | MUST | **Prefer-start wrapper.** Navigation lists wrap the matcher so that a match whose first fragment is at index 0 gets `+10000`. It is applied when the user's pattern (before decoration) does not start with `*`. | cases 07.30 to 07.33 |
| SRCH-MAT-035 | MUST | **Exact-name bonus.** In Go to Class and Go to Symbol an element whose qualified name ends with the typed qualified text (preceded by a separator or a non-identifier char) gets `+5000` (`EXACT_MATCH_DEGREE = 10000 + 5000` counting the prefer-start bonus). Applies only when the typed text contains a separator (`.`, `#`, `::`) and no `*`. | cases 07.40, 07.41 |
| SRCH-MAT-036 | MUST | The matcher is **pure and deterministic**: same pattern, name and flags give the same fragments and degree. It never throws on odd input (empty names, astral code points, surrogate pairs). | property test |

## 5. How lists use the matcher

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| SRCH-MAT-040 | MUST | **Pattern decoration.** Go to Class / File / Symbol and their Search Everywhere tabs match names with the pattern `"*" + typed` when the typed text (trimmed) has more than one character; a single character matches as a prefix only. The prefix `*` is the "middle matching" switch (registry `ide.goto.middle.matching`, default on). The prefer-start bonus of 034 applies when the typed text itself does not start with `*`. | cases 07.30 to 07.33 |
| SRCH-MAT-041 | MUST | **Name sort in a name list**: (1) a name equal to the typed text ignoring case, (2) a name equal to the typed name part ignoring case, (3) higher degree first, (4) name compared ignoring case ascending. After that, elements that share a name are ordered by the list rules of the tab (project proximity, then qualified name). | cases 07.30 to 07.36 |
| SRCH-MAT-042 | MUST | **Line and column suffix.** Before matching, the typed text is reduced to its name part: if it contains one of `: , ; @ [ (` or space or `#`, or the words ` line ` or `?l=`, the first match of `(.+?)` + separator + optional line + optional `\W` column + optional closing `)` or `]` is taken as the name and the rest is the position. Separators: `:`, `@`, `,`, space, `#`, `#L`, `?l=`, ` on line `, ` at line `, `:line `, `:(`, `(`, `:[`, `[`. A trailing space of the original is kept (it means "full match"). `Foo:12`, `Foo:12:5`, `Foo(12)`, `Foo#L12`, `Foo on line 12` are all name `Foo`. | cases 07.50 to 07.58 |
| SRCH-MAT-043 | MUST | **Class and symbol patterns.** For Go to Class a `#` splits `Class#member`: the class part is matched as the name and the member part is applied after opening the class; for Go to Symbol `Class#member` keeps the whole text and only a trailing `(...)` signature is dropped; a `$` pattern like `Outer$1$2` selects anonymous classes by their path (`[.\w]+` followed by `($digits)*` and an optional `$`). | cases 07.53 to 07.56 |
| SRCH-MAT-044 | MUST | **Qualified class and symbol patterns.** If the typed name part contains a separator of the model (`.` for classes and symbols; `#` and `::` where a language adds them), the text after the last separator (ignoring a trailing separator) is the *name part* and finds candidate names exactly as in 040. Each candidate element is then kept only if its **qualified name** matches the whole typed text: the typed text is decorated as `"*" + text` with every separator replaced by `*<sep>*`, and the qualified name has its separators normalised to one fixed char before matching. The qualified match is a filter; the order stays name degree (+ the exact bonus of 035). Elements with the same short name order: exact qualified match first, then closeness to the current file's package or directory, then qualified name ascending. | cases 07.60 to 07.63 |
| SRCH-MAT-045 | MUST | **Qualified file patterns.** In Go to File the text is first normalised: `\` becomes `/`, leading and trailing `/` are removed. The text after the last `/` is the file name part (matched as 040, with the tier rule of 046), the text before is the *directory part*. A file is kept only if its **parent path** (project-relative, with a trailing `/`) matches the directory part decorated as `"*" + part` with every `/` replaced by `*/*` and `\` by `*\*`, start matches preferred. Its weight becomes the degree of that directory match, minus the number of characters of the parent path after the last matched fragment (so shallower, tighter matches rank first), plus the exact bonus when the typed text is a path suffix of the file's path. | cases 07.64 to 07.66 |
| SRCH-MAT-046 | MUST | **Go to File name tiers.** Candidate file names are grouped before ordering: names that **start with the typed name part exactly (same case)** form the top tier and are ordered by file name without extension, lower-cased, ascending; all others are ordered by name degree (descending); names with equal degree form a group whose files are ordered by name without extension (lower-cased), then within one name by closeness to the current file, then fewer directory levels, then path ascending. Directories come after files within a group. | cases 07.68, 07.69 |
| SRCH-MAT-047 | SHOULD | **`*` inside the pattern.** For `a*b*c`, names are also tried against each suffix after a `*` (`b*c`, then `c`) so a qualifier typed before `*` still narrows by container. | case 07.67 |
| SRCH-MAT-048 | SHOULD | **Action patterns** use a different, word-based rule in Search Everywhere; see SRCH-SE-040 to SRCH-SE-046. | |

Open questions: see `04-search-everywhere.md` section 12 (fuzzy and typo-tolerant matching, Pinyin).

Not specified here and left LATER: typo-tolerant matching (Bitap), Pinyin matching, keyboard-layout fixing beyond the fallback pattern of 005, Levenshtein "fuzzy class/file search" (registries `search.everywhere.fuzzy.*`, all default off).
