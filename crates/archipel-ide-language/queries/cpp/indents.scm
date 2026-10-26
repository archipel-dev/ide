; C++ indentation. Access specifiers align with the class body's braces.
[
  (compound_statement)
  (field_declaration_list)
  (enumerator_list)
  (initializer_list)
  (declaration_list)
  (parameter_list)
  (argument_list)
  (case_statement)
  (template_argument_list)
] @indent

["}" ")" "]"] @outdent

(access_specifier) @indent.branch

["{" "(" "["] @indent.after
(case_statement ":" @indent.after)
