; Go indentation: bodies, literals, case clauses.
[
  (block)
  (field_declaration_list)
  (literal_value)
  (import_spec_list)
  (parameter_list)
  (argument_list)
  (expression_case)
  (default_case)
  (type_case)
  (communication_case)
  (interface_type)
] @indent

["}" ")" "]"] @outdent

["{" "(" "["] @indent.after
(expression_case ":" @indent.after)
(default_case ":" @indent.after)
