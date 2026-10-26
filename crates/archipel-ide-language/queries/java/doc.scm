; Javadoc comments; annotations sit between a comment and its declaration.
((block_comment) @doc
  (#match? @doc "^/\\*\\*[^/]"))
(marker_annotation) @doc.skip
(annotation) @doc.skip
