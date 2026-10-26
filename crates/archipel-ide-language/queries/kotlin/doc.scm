; KDoc comments; annotations sit between a comment and its declaration.
((multiline_comment) @doc
  (#match? @doc "^/\\*\\*[^/]"))
