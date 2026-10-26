; Kotlin selection groups and text objects.
[
  (value_arguments)
  (function_value_parameters)
  (class_body)
  (enum_class_body)
  (function_body)
  (control_structure_body)
  (statements)
  (collection_literal)
  (type_arguments)
  (type_parameters)
  (class_parameter)
] @select.group

(function_declaration) @function.outer
(anonymous_function) @function.outer
(lambda_literal) @function.outer
(class_declaration) @class.outer
(object_declaration) @class.outer
(companion_object) @class.outer
(function_value_parameters) @parameter.list
(multiline_comment) @comment.outer
(line_comment) @comment.outer
