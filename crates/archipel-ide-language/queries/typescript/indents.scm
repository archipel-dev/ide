; JavaScript indentation. `case` lines sit one level inside the switch body.
[
  (statement_block)
  (class_body)
  (object)
  (array)
  (switch_body)
  (switch_case)
  (switch_default)
  (arguments)
  (formal_parameters)
  (named_imports)
  (interface_body)
  (enum_body)
  (object_type)
  (template_substitution)
] @indent

["}" ")" "]"] @outdent

["{" "(" "[" "=>"] @indent.after
(switch_case ":" @indent.after)
(switch_default ":" @indent.after)
