; C indentation: `case` lines sit at the switch's level, their statements one in.
[
  (compound_statement)
  (field_declaration_list)
  (enumerator_list)
  (initializer_list)
  (parameter_list)
  (argument_list)
  (case_statement)
] @indent

["}" ")" "]"] @outdent

["{" "(" "["] @indent.after
(case_statement ":" @indent.after)
