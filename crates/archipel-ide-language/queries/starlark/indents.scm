; Starlark indentation: definitions and control flow indent their bodies.
[
  (function_definition)
  (if_statement)
  (elif_clause)
  (else_clause)
  (for_statement)
  (dictionary)
  (list)
  (tuple)
  (argument_list)
  (parameters)
  (parenthesized_expression)
  (list_comprehension)
] @indent

["}" ")" "]"] @outdent

[(elif_clause) (else_clause)] @indent.branch

(function_definition ":" @indent.after)
(if_statement ":" @indent.after)
(elif_clause ":" @indent.after)
(else_clause ":" @indent.after)
(for_statement ":" @indent.after)
["(" "[" "{"] @indent.after

[(return_statement) (pass_statement) (break_statement) (continue_statement)] @indent.stop
