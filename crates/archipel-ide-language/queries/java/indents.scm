; Java indentation. `case` lines sit one level inside the switch block, their
; statements one more.
[
  (block)
  (class_body)
  (interface_body)
  (enum_body)
  (annotation_type_body)
  (constructor_body)
  (switch_block)
  (switch_block_statement_group)
  (switch_rule)
  (array_initializer)
  (argument_list)
  (formal_parameters)
  (element_value_array_initializer)
] @indent

["}" ")" "]"] @outdent

["{" "(" "["] @indent.after
(switch_label) @indent.after
