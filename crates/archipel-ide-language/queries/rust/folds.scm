; Rust folding: bodies and lists with braces/brackets, macro bodies, imports,
; block and doc comments, `region` markers.
[
  (block)
  (declaration_list)
  (field_declaration_list)
  (enum_variant_list)
  (match_block)
  (field_initializer_list)
  (array_expression)
  (token_tree)
] @fold

((use_declaration) @fold.imports
  (#set! collapsed "true"))

(block_comment) @fold.comment

; A run of `///` lines folds as one documentation comment.
((line_comment (outer_doc_comment_marker)) @fold.doc.run)

(line_comment) @fold.marker
