; Rust indentation: containers indent their contents by one level, closing
; tokens align with the line that opened the container.
[
  (block)
  (declaration_list)
  (field_declaration_list)
  (enum_variant_list)
  (match_block)
  (field_initializer_list)
  (arguments)
  (parameters)
  (array_expression)
  (tuple_expression)
  (token_tree)
  (type_arguments)
] @indent

["}" ")" "]"] @outdent

["{" "(" "["] @indent.after
