; Starlark folding: indented bodies and multi-line literals and calls.
[
  (block)
  (dictionary)
  (list)
  (tuple)
  (argument_list)
  (parameters)
  (list_comprehension)
  (parenthesized_expression)
] @fold

((expression_statement (call function: (identifier) @_f) @fold.imports)
  (#eq? @_f "load")
  (#set! collapsed "true"))

(expression_statement (string) @fold.doc)
(comment) @fold.marker
