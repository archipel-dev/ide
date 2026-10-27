# Cases for 01-find-replace-in-file.md

Conventions: text blocks use `<caret>` and `<selection>…</selection>` (a selection made by a search has its caret at the end,
written `<selection>foo<caret></selection>`). Lines that start with `bar:`, `hint:`, `status:` or `dialog:` in an `after` block describe
non-text state and are checked in the order given. `keys:` lists key presses and typed text; `Esc`, `Enter`, `Tab` are keys; the
default keymap is assumed. "field" is the search field, "replace field" the second row. Unless a case says otherwise the bar options
are at their defaults (Match Case, Words, Regex off; filter Anywhere; scroll to results during typing on).

### SRCH-FIL-002 case 1: Ctrl+F opens the bar in find mode
settings: default
language: plain
```before
hello <caret>world
```
keys: Ctrl+F
```after
hello <caret>world
bar: visible above the editor text, find mode (no replace field), search field focused and empty, counter "0 results"
```

### SRCH-FIL-002 case 2: Ctrl+R opens the bar in replace mode
settings: default
language: plain
```before
hello <caret>world
```
keys: Ctrl+R
```after
hello <caret>world
bar: visible, replace field shown below the search field, search field focused
```

### SRCH-FIL-002 case 3: Ctrl+F with the bar open in replace mode switches to find mode and selects the text
settings: default
language: plain
```before
foo bar <caret>foo
```
keys: Ctrl+R, type "foo", Ctrl+F
```after
foo bar <selection>foo<caret></selection>
bar: find mode (replace field hidden), search field focused with its text "foo" selected, counter "2/2"
```

### SRCH-FIL-003 case 1: Esc closes the bar and keeps the last match selected
settings: default
language: plain
```before
one <caret>two three two
```
keys: Ctrl+F, type "two", Esc
```after
one <selection>two<caret></selection> three two
bar: closed, focus in the editor
```

### SRCH-FIL-003 case 2: Ctrl+Enter keeps the bar, focuses the editor; on an empty field it closes
settings: default
language: plain
```before
one two <caret>one
```
keys: Ctrl+F, type "one", Ctrl+Enter
```after
one two <selection>one<caret></selection>
bar: still visible, focus in the editor, "one" is the first entry of the search history
```

### SRCH-FIL-003 case 3: Ctrl+Enter on an empty field closes the bar
settings: default
language: plain
```before
one <caret>two
```
keys: Ctrl+F, Ctrl+Enter
```after
one <caret>two
bar: closed
```

### SRCH-FIL-010 case 1: counter with a current match
settings: default
language: plain
```before
<caret>a1 b22 c333
```
keys: Ctrl+F, Alt+X, type "\d+"
```after
a<selection>1<caret></selection> b22 c333
bar: counter "1/3", regex toggle on
```

### SRCH-FIL-010 case 2: empty field
settings: default
language: plain
```before
<caret>abc
```
keys: Ctrl+F
```after
<caret>abc
bar: counter "0 results", no highlights
```

### SRCH-FIL-010 case 3: invalid regular expression
settings: default
language: plain
```before
<caret>a(b
```
keys: Ctrl+F, Alt+X, type "("
```after
<caret>a(b
bar: counter text "Bad pattern", field in error state, no highlights, Enter does nothing
```

### SRCH-FIL-010 case 4: vague pattern
settings: default
language: plain
```before
<caret>a|b
```
keys: Ctrl+F, Alt+X, type "||"
```after
<caret>a|b
bar: counter text "Vague pattern", no highlights
```

### SRCH-FIL-010 case 5: search in selection with no selection
settings: default
language: plain
```before
<caret>x y z
```
keys: Ctrl+F, Ctrl+Alt+E, type "x"
```after
<caret>x y z
bar: counter text "No selection", the "Search in Selection" toggle on, field placeholder (when empty) "Search in selection only"
```

### SRCH-FIL-010 case 6: more than 10000 matches
settings: default
language: plain
```before
<caret>(the text "a" repeated 20000 times)
```
keys: Ctrl+F, type "a"
```after
bar: counter "10000+", a link "Click to highlight" is shown; activating the link changes the counter to "1/20000"
```

### SRCH-FIL-011 case 1: nothing found
settings: default
language: plain
```before
<caret>abc
```
keys: Ctrl+F, type "zzz"
```after
<caret>abc
bar: counter "0 results", search field in the not-found (error) state
```

### SRCH-FIL-011 case 2: the error state clears when text is deleted
settings: default
language: plain
```before
<caret>abc
```
keys: Ctrl+F, type "zzz", Backspace, Backspace, Backspace
```after
<caret>abc
bar: counter "0 results", search field in the normal state
```

### SRCH-FIL-012 case 1: results follow every keystroke
settings: default
language: plain
```before
<caret>abc abd abe
```
keys: Ctrl+F, type "ab", type "d"
```after
abc <selection>abd<caret></selection> abe
bar: counter "1/1" after "abd"; after the first typed "ab" the counter was "1/3"
```

### SRCH-FIL-013 case 1: the editor selection follows the first match while typing
settings: scroll-to-results-during-typing=on
language: plain
```before
alpha
beta foo
gamma <caret>
foo delta
```
keys: Ctrl+F, type "foo"
```after
alpha
beta foo
gamma 
<selection>foo<caret></selection> delta
bar: counter "2/2"
```

### SRCH-FIL-013 case 2: with the setting off only highlighting happens until the user navigates
settings: scroll-to-results-during-typing=off
language: plain
```before
<caret>x foo foo
```
keys: Ctrl+F, type "foo", Enter
```after
x <selection>foo<caret></selection> foo
bar: counter "1/2"
```

### SRCH-FIL-014 case 1: a single-line selection becomes the search text
settings: default
language: plain
```before
<selection>foo</selection> bar foo
```
keys: Ctrl+F
```after
<selection>foo<caret></selection> bar foo
bar: field text "foo" (all selected), counter "1/2"
```

### SRCH-FIL-014 case 2: a multi-line selection switches to search in selection
settings: default
language: plain
```before
<selection>one
two</selection>
```
keys: Ctrl+F
```after
bar: field empty with placeholder "Search in selection. Ctrl+Alt+E to search for selected text", "Search in Selection" toggle on
```

### SRCH-FIL-014 case 3: reopening shows the previous text selected
settings: default
language: plain
```before
<caret>foo bar foo
```
keys: Ctrl+F, type "foo", Esc, Ctrl+F
```after
bar: visible, field text "foo" with all of it selected
```

### SRCH-FIL-014 case 4: a copied selection is regex-escaped when regex is on
settings: regex-toggle=on (remembered from the last session)
language: plain
```before
<selection>a.b</selection> axb
```
keys: Ctrl+F
```after
bar: field text "a\.b", counter "1/1"
```

### SRCH-FIL-015 case 1: toggles are remembered between sessions
settings: default
language: plain
```before
<caret>Foo foo
```
keys: Ctrl+F, Alt+C, Esc, Ctrl+F, type "foo"
```after
Foo <selection>foo<caret></selection>
bar: "Match Case" still on, counter "1/1"
```

### SRCH-FIL-020 case 1: case-insensitive by default
settings: default
language: plain
```before
<caret>Foo foo FOO fOo
```
keys: Ctrl+F, type "foo"
```after
<selection>Foo<caret></selection> foo FOO fOo
bar: counter "1/4"
```

### SRCH-FIL-020 case 2: Match Case
settings: default
language: plain
```before
<caret>Foo foo FOO fOo
```
keys: Ctrl+F, Alt+C, type "foo"
```after
Foo <selection>foo<caret></selection> FOO fOo
bar: counter "1/1"
```

### SRCH-FIL-020 case 3: occurrences do not overlap
settings: default
language: plain
```before
<caret>aaaa
```
keys: Ctrl+F, type "aa"
```after
<selection>aa<caret></selection>aa
bar: counter "1/2"
```

### SRCH-FIL-021 case 1: whole words and identifier characters
settings: default
language: plain
```before
<caret>foo foobar barfoo foo_bar foo.bar
foo
```
keys: Ctrl+F, Alt+W, type "foo"
```after
<selection>foo<caret></selection> foobar barfoo foo_bar foo.bar
foo
bar: counter "1/3" (matches: the first foo, the foo before ".bar", the foo on line 2)
```

### SRCH-FIL-021 case 2: a word after an escape sequence
settings: default
language: plain
```before
<caret>s = "a\nfoo"
```
keys: Ctrl+F, Alt+W, type "foo"
```after
s = "a\n<selection>foo<caret></selection>"
bar: counter "1/1"
```

### SRCH-FIL-021 case 3: non-identifier search text
settings: default
language: plain
```before
<caret>a == b === c
```
keys: Ctrl+F, Alt+W, type "=="
```after
a <selection>==<caret></selection> b === c
bar: counter "1/1" (the pairs inside "===" are rejected)
```

### SRCH-FIL-022 case 1: regex digits
settings: default
language: plain
```before
<caret>a1 b22 c333
```
keys: Ctrl+F, Alt+X, type "\d+"
```after
a<selection>1<caret></selection> b22 c333
bar: counter "1/3"
```

### SRCH-FIL-022 case 2: MULTILINE anchors
settings: default
language: plain
```before
<caret>ab
cd
ef
```
keys: Ctrl+F, Alt+X, type "^\w"
```after
<selection>a<caret></selection>b
cd
ef
bar: counter "1/3"
```

### SRCH-FIL-022 case 3: lookbehind
settings: default
language: plain
```before
<caret>ab cb ab
```
keys: Ctrl+F, Alt+X, type "(?<=a)b"
```after
a<selection>b<caret></selection> cb ab
bar: counter "1/2"
```

### SRCH-FIL-022 case 4: regex is case-insensitive unless Match Case
settings: default
language: plain
```before
<caret>Foo foo foobar
```
keys: Ctrl+F, Alt+X, type "\bfoo\b"
```after
<selection>Foo<caret></selection> foo foobar
bar: counter "1/2"
```

### SRCH-FIL-022 case 5: a pattern with \n spans lines
settings: default
language: plain
```before
<caret>a
b a b
```
keys: Ctrl+F, Alt+X, type "a\nb"
```after
<selection>a
b<caret></selection> a b
bar: counter "1/1"
```

### SRCH-FIL-022 case 6: Words applies on top of regex matches
settings: default
language: plain
```before
<caret>ab abc ab_
```
keys: Ctrl+F, Alt+X, Alt+W, type "ab"
```after
<selection>ab<caret></selection> abc ab_
bar: counter "1/1"
```

### SRCH-FIL-023 case 1: a regex that matches only the empty string
settings: default
language: plain
```before
<caret>abc
```
keys: Ctrl+F, Alt+X, type "x*"
```after
<caret>abc
bar: counter "0 results"
```

### SRCH-FIL-024 case 1: multi-line search text
settings: default
language: plain
```before
<caret>foo
bar foo
bar
```
keys: Ctrl+F, paste "foo\nbar"
```after
<selection>foo
bar<caret></selection> foo
bar
bar: counter "1/2", the search field shows two rows
```

### SRCH-FIL-025 case 1: search in selection counts only matches inside
settings: default
language: plain
```before
foo
<selection>foo
foo
</selection>foo
```
keys: Ctrl+F, type "foo"
```after
bar: "Search in Selection" toggle on; total matches 2 (the two inside the selection); the selection is unchanged
```

### SRCH-FIL-025 case 2: turning the toggle off restores the initial selection
settings: default
language: plain
```before
foo
<selection>foo
foo
</selection>foo
```
keys: Ctrl+F, type "foo", Ctrl+Alt+E
```after
bar: "Search in Selection" toggle off; total matches 4
```

### SRCH-FIL-030 case 1: Enter and Shift+Enter move between matches
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+F, type "foo", Enter
```after
foo <selection>foo<caret></selection> foo
bar: counter "2/3"
```

### SRCH-FIL-030 case 2: Shift+Enter goes back
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+F, type "foo", Enter, Enter, Shift+Enter
```after
foo <selection>foo<caret></selection> foo
bar: counter "2/3"
```

### SRCH-FIL-030 case 3: Down arrow in a single-line field is Next
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+F, type "foo", Down
```after
foo <selection>foo<caret></selection> foo
bar: counter "2/3"
```

### SRCH-FIL-031 case 1: no silent wrap-around forwards
settings: default
language: plain
```before
<caret>foo foo
```
keys: Ctrl+F, type "foo", Enter, Enter
```after
foo <selection>foo<caret></selection>
bar: counter "2/2"
hint: under the caret: "foo" not found, press F3 to search from the top
```

### SRCH-FIL-031 case 2: the next press wraps
settings: default
language: plain
```before
<caret>foo foo
```
keys: Ctrl+F, type "foo", Enter, Enter, Enter
```after
<selection>foo<caret></selection> foo
bar: counter "1/2"
```

### SRCH-FIL-031 case 3: backwards at the first match
settings: default
language: plain
```before
<caret>foo foo
```
keys: Ctrl+F, type "foo", Shift+Enter
```after
<selection>foo<caret></selection> foo
bar: counter "1/2"
hint: above the caret: "foo" not found, press Shift+F3 to search from the bottom
```

### SRCH-FIL-032 case 1: first match at or after the caret
settings: default
language: plain
```before
foo one
two <caret>three
foo four
```
keys: Ctrl+F, type "foo"
```after
foo one
two three
<selection>foo<caret></selection> four
bar: counter "2/2"
```

### SRCH-FIL-032 case 2: no match after the caret, the first match without a hint
settings: default
language: plain
```before
foo x<caret> y
```
keys: Ctrl+F, type "foo"
```after
<selection>foo<caret></selection> x y
bar: counter "1/1"
hint: none
```

### SRCH-FIL-033 case 1: F3 with the bar closed
settings: default
language: plain
```before
<caret>foo a foo b foo
```
keys: Ctrl+F, type "foo", Esc, F3, F3
```after
foo a foo b <selection>foo<caret></selection>
bar: closed
```

### SRCH-FIL-033 case 2: F3 at the end shows the hint, the next F3 wraps
settings: default
language: plain
```before
<caret>foo a foo
```
keys: Ctrl+F, type "foo", Esc, F3, F3, F3
```after
<selection>foo<caret></selection> a foo
hint: shown after the second F3: "foo" not found, press F3 to search from the top
```

### SRCH-FIL-033 case 3: Shift+F3 with the bar closed
settings: default
language: plain
```before
foo a foo b <caret>foo
```
keys: Ctrl+F, type "foo", Esc, Shift+F3
```after
foo a <selection>foo<caret></selection> b foo
```

### SRCH-FIL-034 case 1: Ctrl+F3 finds the next occurrence of the word at the caret
settings: default
language: plain
```before
foo<caret> x foo y foo
```
keys: Ctrl+F3
```after
foo x <selection>foo<caret></selection> y foo
```

### SRCH-FIL-034 case 2: case-sensitive, substring
settings: default
language: plain
```before
foo<caret> Foo foobar
```
keys: Ctrl+F3
```after
foo Foo <selection>foo<caret></selection>bar
```

### SRCH-FIL-034 case 3: not found, then F3 wraps
settings: default
language: plain
```before
foo<caret> x
```
keys: Ctrl+F3, F3
```after
<selection>foo<caret></selection> x
hint: after the first key: "foo" not found, press F3 to search from the top
```

### SRCH-FIL-034 case 4: no word at the caret
settings: default
language: plain
```before
a <caret> b
```
keys: Ctrl+F3
```after
a <caret> b
```

### SRCH-FIL-034 case 5: backwards
settings: default
language: plain
```before
foo x foo<caret> y
```
keys: Ctrl+Shift+F3
```after
<selection>foo<caret></selection> x foo y
```

### SRCH-FIL-040 case 1: Replace replaces the current match and selects the next
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+R, type "foo", Tab, type "bar", Enter
```after
bar <selection>foo<caret></selection> foo
bar: counter "1/2"
```

### SRCH-FIL-040 case 2: Replace is one undo step
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+R, type "foo", Tab, type "bar", Enter, Esc, Ctrl+Z
```after
foo foo foo
```

### SRCH-FIL-041 case 1: Replace All
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+R, type "foo", Tab, type "x", Alt+A
```after
x x x
status: 3 occurrences replaced
bar: still visible, counter "0 results"
```

### SRCH-FIL-041 case 2: the replacement is not scanned again
settings: default
language: plain
```before
<caret>aaa
```
keys: Ctrl+R, type "a", Tab, type "aa", Alt+A
```after
aaaaaa
status: 3 occurrences replaced
```

### SRCH-FIL-041 case 3: Replace All in the selection only
settings: default
language: plain
```before
foo
<selection>foo
foo
</selection>foo
```
keys: Ctrl+R, type "foo", Tab, type "x", Alt+A
```after
foo
x
x
foo
status: 2 occurrences replaced (the selection after the replacement is not specified)
```

### SRCH-FIL-041 case 4: Replace All is one undo step and one Local History entry
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+R, type "foo", Tab, type "x", Alt+A, Esc, Ctrl+Z
```after
foo foo foo
status: Local History contains the action "Replace all 'foo' in file with 'x'"
```

### SRCH-FIL-041 case 5: one occurrence
settings: default
language: plain
```before
<caret>foo bar
```
keys: Ctrl+R, type "foo", Tab, type "x", Alt+A
```after
x bar
status: 1 occurrence replaced
```

### SRCH-FIL-041 case 6: case-insensitive find replaces every spelling by the literal text
settings: default
language: plain
```before
<caret>Foo foo FOO
```
keys: Ctrl+R, type "foo", Tab, type "x", Alt+A
```after
x x x
```

### SRCH-FIL-042 case 1: Exclude keeps a match out of Replace All
settings: default
language: plain
```before
<caret>foo foo foo
```
keys: Ctrl+R, type "foo", Tab, type "x", Alt+L, Alt+A
```after
foo x x
bar: the Exclude button text is "Exclude" again after Replace All
```

### SRCH-FIL-043 case 1: group references
settings: regex=on
language: plain
```before
<caret>a.b
```
keys: Ctrl+R, type "(\w+)\.(\w+)", Tab, type "$2.$1", Alt+A
```after
b.a
```

### SRCH-FIL-043 case 2: named groups
settings: regex=on
language: plain
```before
<caret>x=y
```
keys: Ctrl+R, type "(?<k>\w+)=(?<v>\w+)", Tab, type "${v}=${k}", Alt+A
```after
y=x
```

### SRCH-FIL-043 case 3: \U and \E
settings: regex=on
language: plain
```before
<caret>hello world
```
keys: Ctrl+R, type "(\w+) (\w+)", Tab, type "\U$1\E $2", Alt+A
```after
HELLO world
```

### SRCH-FIL-043 case 4: \u upper-cases only the next character
settings: regex=on
language: plain
```before
<caret>foo bar
```
keys: Ctrl+R, type "\b(\w+)", Tab, type "\u$1", Alt+A
```after
Foo Bar
```

### SRCH-FIL-043 case 5: \L without \E runs to the end of the replacement
settings: regex=on
language: plain
```before
<caret>ABC DEF
```
keys: Ctrl+R, type "(\w+)", Tab, type "\L$1", Alt+A
```after
abc def
```

### SRCH-FIL-043 case 6: \n and \t
settings: regex=on
language: plain
```before
<caret>a,b
```
keys: Ctrl+R, type ",", Tab, type "\n", Alt+A
```after
a
b
```

### SRCH-FIL-043 case 7: $0 and a group that did not participate
settings: regex=on
language: plain
```before
<caret>a1b22 ab
```
keys: Ctrl+R, type "(a)|(b)", Tab, type "[$1$2]", Alt+A
```after
[a]1[b]22 [a][b]
```

### SRCH-FIL-043 case 8: a two-digit reference that is not a group is a one-digit group and a digit
settings: regex=on
language: plain
```before
<caret>ab
```
keys: Ctrl+R, type "(a)(b)", Tab, type "$12", Alt+A
```after
a2
```

### SRCH-FIL-043 case 9: escaped dollar and backslash
settings: regex=on
language: plain
```before
<caret>x y
```
keys: Ctrl+R, type "x", Tab, type "\$\\", Alt+A
```after
$\ y
```

### SRCH-FIL-043 case 10: without regex the replacement is literal
settings: default
language: plain
```before
<caret>a
```
keys: Ctrl+R, type "a", Tab, type "$1\n", Alt+A
```after
$1\n
```

### SRCH-FIL-044 case 1: dollar at the end
settings: regex=on
language: plain
```before
<caret>a
```
keys: Ctrl+R, type "(a)", Tab, type "x$", Alt+A
```after
a
dialog: title "Replace Error", message "Malformed replacement string: Illegal group reference: group index is missing"; text unchanged
```

### SRCH-FIL-044 case 2: unknown group number
settings: regex=on
language: plain
```before
<caret>a
```
keys: Ctrl+R, type "(a)", Tab, type "$3", Alt+A
```after
a
dialog: title "Replace Error", message "Malformed replacement string: No group 3"; text unchanged
```

### SRCH-FIL-044 case 3: trailing backslash
settings: regex=on
language: plain
```before
<caret>a
```
keys: Ctrl+R, type "a", Tab, type "x\", Alt+A
```after
a
dialog: title "Replace Error", message "Malformed replacement string: character to be escaped is missing"; text unchanged
```

### SRCH-FIL-045 case 1: preserve case on whole words and parts
settings: preserve-case=on
language: plain
```before
<caret>Foo foo FOO fooBar
```
keys: Ctrl+R, type "foo", Tab, type "bar", Alt+E, Alt+A
```after
Bar bar BAR barBar
```

### SRCH-FIL-045 case 2: preserve case with a longer replacement word
settings: preserve-case=on
language: plain
```before
<caret>get Get GET
```
keys: Ctrl+R, type "get", Tab, type "fetch", Alt+E, Alt+A
```after
fetch Fetch FETCH
```

### SRCH-FIL-045 case 3: identifiers with underscores and a regex find
settings: regex=on, preserve-case=on
language: plain
```before
<caret>UserId user_id USERID
```
keys: Ctrl+R, type "user_?id", Tab, type "account_id", Alt+E, Alt+A
```after
Account_Id account_id ACCOUNT_ID
```

### SRCH-FIL-045 case 4: several words
settings: preserve-case=on
language: plain
```before
<caret>Hello World hello world HELLO WORLD
```
keys: Ctrl+R, type "hello world", Tab, type "good morning", Alt+E, Alt+A
```after
Good Morning good morning GOOD MORNING
```

### SRCH-FIL-045 case 5: a found text without letters leaves the replacement unchanged
settings: preserve-case=on
language: plain
```before
<caret>111
```
keys: Ctrl+R, type "111", Tab, type "Abc", Alt+E, Alt+A
```after
Abc
```

### SRCH-FIL-045 case 6: preserve case off by default
settings: default
language: plain
```before
<caret>Foo FOO
```
keys: Ctrl+R, type "foo", Tab, type "bar", Alt+A
```after
bar bar
```

### SRCH-FIL-050 case 1: In Comments
settings: filter=In Comments
language: java
```before
// foo
class A { int foo = 0; /* foo */ String s = "foo"; }
```
keys: Ctrl+F, open the filter menu and choose "In Comments", type "foo"
```after
bar: total matches 2 (the two inside comments); the filter button is highlighted
```

### SRCH-FIL-050 case 2: Except Comments and String Literals
settings: filter=Except Comments and String Literals
language: java
```before
// foo
class A { int foo = 0; /* foo */ String s = "foo"; }
```
keys: Ctrl+F, choose "Except Comments and String Literals", type "foo"
```after
bar: total matches 1 (the field name)
```

### SRCH-FIL-050 case 3: In String Literals
settings: filter=In String Literals
language: java
```before
// foo
class A { int foo = 0; /* foo */ String s = "foo"; }
```
keys: Ctrl+F, choose "In String Literals", type "foo"
```after
bar: total matches 1 (inside "foo")
```

### SRCH-FIL-060 case 1: Select All Occurrences
settings: default
language: plain
```before
<caret>foo bar foo
```
keys: Ctrl+F, type "foo", Ctrl+Alt+Shift+J
```after
<selection>foo<caret></selection> bar <selection>foo<caret></selection>
bar: closed
```

### SRCH-FIL-061 case 1: add the next occurrence from the bar
settings: default
language: plain
```before
<caret>foo bar foo baz foo
```
keys: Ctrl+F, type "foo", Alt+J
```after
<selection>foo<caret></selection> bar <selection>foo<caret></selection> baz foo
bar: still visible
```

### SRCH-FIL-070 case 1: history is newest first and deduplicated
settings: default
language: plain
```before
<caret>foo bar
```
keys: Ctrl+F, type "foo", Enter, Ctrl+A, type "bar", Enter, Ctrl+A, type "foo", Enter, Alt+Down
```after
dialog: history popup with the entries, top to bottom: foo, bar
```

### SRCH-FIL-070 case 2: choosing an entry searches
settings: default
language: plain
```before
<caret>foo bar
```
keys: Ctrl+F, type "foo", Enter, Ctrl+A, type "bar", Enter, Alt+Down, Down, Enter
```after
<selection>foo<caret></selection> bar
bar: field text "foo"
```

### SRCH-FIL-071 case 1: completion of words from the file
settings: default
language: plain
```before
<caret>HashMap HashSet Hashtable
```
keys: Ctrl+F, type "hm", Ctrl+Space
```after
dialog: completion popup with the single entry "HashMap"
```
