; Java selection groups and text objects.
[
  (argument_list)
  (formal_parameters)
  (array_initializer)
  (element_value_array_initializer)
  (block)
  (class_body)
  (interface_body)
  (enum_body)
  (constructor_body)
  (switch_block)
  (type_arguments)
  (type_parameters)
  (inferred_parameters)
  (type_list)
] @select.group

(method_declaration) @function.outer
(method_declaration body: (block) @function.inner)
(constructor_declaration) @function.outer
(lambda_expression) @function.outer
(class_declaration) @class.outer
(interface_declaration) @class.outer
(enum_declaration) @class.outer
(record_declaration) @class.outer
(formal_parameters) @parameter.list
(block_comment) @comment.outer
(line_comment) @comment.outer
