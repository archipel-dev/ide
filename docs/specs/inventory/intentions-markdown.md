# Intentions: markdown

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Intentions (13)

Description = first sentence of the shipped description resource, if any.

| Class | Language | Category | Description | Spec |
|---|---|---|---|---|
| `ConvertToInlineLinkIntention` | Markdown | Markdown | Replaces a reference-style link with an inline link that holds the address, and removes the link definition. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToReferenceLinkIntention` | Markdown | Markdown | Replaces an inline link with a reference-style link, and adds the link definition at the end of the file. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `FixCellAlignmentIntention` | Markdown | Markdown | Adjusts the current table cell to match its column alignment. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `FixTableBordersIntention` | Markdown | Markdown | Adds side borders to the current Markdown table. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `InsertAfter` | Markdown | Markdown |  | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `InsertBefore` | Markdown | Markdown |  | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MarkdownCodeFenceErrorHighlightingIntention` | Markdown | Markdown | Allows disabling error highlighting in Markdown code fences. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MarkdownConfigureImageIntention` | Markdown | Markdown | Opens a dialog for interactive configuration of Markdown images. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MarkdownRemoveColumnIntention` | Markdown | Markdown | Removes current column. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MarkdownRemoveRowIntention` | Markdown | Markdown | Removes current table row. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MarkdownSetColumnAlignmentIntention` | Markdown | Markdown | Update column alignment. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `OmitHeadingFromTocIntention` | Markdown | Markdown | Adds a comment that keeps the current heading out of the generated table of contents. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReformatTableIntention` | Markdown | Markdown | Formats all columns in the current Markdown table. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
