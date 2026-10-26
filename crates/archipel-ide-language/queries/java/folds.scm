; Java folding: class bodies, blocks, array and switch bodies, imports,
; block and Javadoc comments, `region` markers.
[
  (block)
  (class_body)
  (interface_body)
  (enum_body)
  (annotation_type_body)
  (constructor_body)
  (switch_block)
  (array_initializer)
  (element_value_array_initializer)
] @fold

((import_declaration) @fold.imports
  (#set! collapsed "true"))

(block_comment) @fold.comment
(line_comment) @fold.marker
