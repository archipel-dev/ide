; Groovy indentation.
[
  (block)
  (class_body)
  (closure)
  (map_literal)
  (list_literal)
  (enum_body)
  (formal_parameters)
  (argument_list)
] @indent

["}" ")" "]"] @outdent

["{" "(" "["] @indent.after
