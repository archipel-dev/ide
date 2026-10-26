; Kotlin indentation.
[
  (class_body)
  (enum_class_body)
  (function_body)
  (control_structure_body)
  (lambda_literal)
  (when_expression)
  (value_arguments)
  (function_value_parameters)
  (primary_constructor)
  (collection_literal)
] @indent

["}" ")" "]"] @outdent

["{" "(" "[" "->"] @indent.after
