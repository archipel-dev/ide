; C selection groups and text objects.
[
  (argument_list)
  (parameter_list)
  (initializer_list)
  (compound_statement)
  (field_declaration_list)
  (enumerator_list)
] @select.group

(function_definition) @function.outer
(function_definition body: (compound_statement) @function.inner)
(struct_specifier) @class.outer
(enum_specifier) @class.outer
(union_specifier) @class.outer
(parameter_list) @parameter.list
(comment) @comment.outer
