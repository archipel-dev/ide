; JavaScript folding: bodies, literals, switch bodies, JSX elements, imports, comments.
[
  (statement_block)
  (class_body)
  (object)
  (array)
  (switch_body)
  (arguments)
  (formal_parameters)
  (named_imports)
] @fold

((import_statement) @fold.imports
  (#set! collapsed "true"))

(comment) @fold.comment
(comment) @fold.marker
[
  (interface_body)
  (object_type)
  (enum_body)
] @fold
