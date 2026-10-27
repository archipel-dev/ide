# Editor acceptance cases: format

One file per spec file, named `<NN>-<name>.cases.md`. Each case is a block:

    ### ED-XXX-NNN case <n>: <short title>
    settings: <key=value, ...> or default
    language: <plain, java, kotlin, ...>
    ```before ... ```
    keys: <key sequence, `type "x"`, or `action <ActionId>`>
    ```after ... ```

Conventions:

* `<caret>` marks a caret. `<selection>...</selection>` marks a selection; unless a `<caret>` appears
  inside it, the caret is at the end of the selection.
* With several carets, the caret listed first in the text is the **oldest**, the last one is the
  **primary** (most recently added).
* Keys use the default Windows/Linux keymap. Several steps are separated by commas. `type "x"` types
  characters one at a time as a user would (typed handlers run).
* `settings:` names are the labels of the settings page or the registry/defaults named in the
  spec; only non-default values are listed. `caret-stop=windows` means the Windows caret stop
  policy of ED-CARET section 3.2; `viewport-lines=N` fixes the viewport height; `line-separator=CRLF`
  means the file on disk uses CRLF (the document always holds `\n`).
* `\t` in a block means a tab character; `·` is not used, spaces are literal.
* `paste "<text>"` puts the text on the clipboard (as plain text, `\n` is a line break) and presses
  Ctrl+V. `copy` and `cut` press Ctrl+C and Ctrl+X. `clipboard: <text>` may appear before `keys:` to
  preset the clipboard.
* A language of `java` means a `.java` file whose text is a complete compilation unit or a class body
  as shown; the caret context decides the typed handlers.
* An optional line `status: unverified (<reason>)` after `settings:` marks a case whose result was
  derived from reading the source but not confirmed in a running IDE (most often caret placement).
  Implementers still implement it; reviewers confirm it first.
* `template: <abbreviation> = <text>` after `settings:` defines a user live template (context: everywhere,
  expand with the default key) that exists before the keys are pressed. `choose "<text>"` means pick the
  popup entry with that text.
* `viewport: N lines, top=K` after `language:` fixes the visible window (zero-based first line K); an
  `after viewport: top=K2` line after the `after` block states the expected first visible line.
* Tab and window cases (file 08) have no text blocks: `tabs: a | [b] | c` gives the tab strip before the
  keys (brackets mark the active tab, `(p)` a pinned tab, `~` a preview tab) and `after tabs: ...` gives it
  afterwards. `open X`, `close X`, `history: a, b, c` (least to most recently used) and `after font size: N`
  are further steps or checks.
