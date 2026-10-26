; Python indentation: compound statements indent their bodies, bracket
; containers their elements; `else`/`elif`/`except`/`finally` align with the
; statement they continue; the next line after `return` dedents.
[
  (function_definition)
  (class_definition)
  (if_statement)
  (elif_clause)
  (else_clause)
  (for_statement)
  (while_statement)
  (try_statement)
  (except_clause)
  (finally_clause)
  (with_statement)
  (match_statement)
  (case_clause)
  (dictionary)
  (list)
  (set)
  (tuple)
  (argument_list)
  (parameters)
  (parenthesized_expression)
  (list_comprehension)
  (dictionary_comprehension)
] @indent

["}" ")" "]"] @outdent


(function_definition ":" @indent.after)
(class_definition ":" @indent.after)
(if_statement ":" @indent.after)
(elif_clause ":" @indent.after)
(else_clause ":" @indent.after)
(for_statement ":" @indent.after)
(while_statement ":" @indent.after)
(try_statement ":" @indent.after)
(except_clause ":" @indent.after)
(finally_clause ":" @indent.after)
(with_statement ":" @indent.after)
(match_statement ":" @indent.after)
(case_clause ":" @indent.after)
["(" "[" "{"] @indent.after

[
  (return_statement)
  (pass_statement)
  (break_statement)
  (continue_statement)
  (raise_statement)
] @indent.stop
