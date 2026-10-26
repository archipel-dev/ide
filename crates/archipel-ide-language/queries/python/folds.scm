; Python folding: indented bodies fold after their header line, docstrings,
; imports, multi-line literals and calls.
[
  (block)
  (dictionary)
  (list)
  (set)
  (tuple)
  (argument_list)
  (parameters)
  (list_comprehension)
  (dictionary_comprehension)
  (parenthesized_expression)
] @fold

((import_statement) @fold.imports
  (#set! collapsed "true"))
((import_from_statement) @fold.imports
  (#set! collapsed "true"))

(expression_statement (string) @fold.doc)
(comment) @fold.marker
