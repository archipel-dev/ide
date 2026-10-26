; Groovy folding: bodies, closures, literals, imports, comments.
[
  (block)
  (class_body)
  (closure)
  (map_literal)
  (list_literal)
  (enum_body)
] @fold

((import_declaration) @fold.imports
  (#set! collapsed "true"))

(block_comment) @fold.comment
(groovydoc_comment) @fold.doc
(line_comment) @fold.marker
