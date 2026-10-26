; Doc comments: `/** */` blocks and `///` lines.
((comment) @doc
  (#match? @doc "^(/\\*\\*[^/]|///)"))
