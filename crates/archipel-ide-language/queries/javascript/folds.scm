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
  (jsx_element)
  (template_string)
] @fold

((import_statement) @fold.imports
  (#set! collapsed "true"))

(comment) @fold.comment
(comment) @fold.marker
