; Kotlin folding: bodies, lambdas, `when`, argument lists, imports, comments.
[
  (class_body)
  (function_body)
  (enum_class_body)
  (control_structure_body)
  (lambda_literal)
  (when_expression)
  (value_arguments)
  (function_value_parameters)
  (primary_constructor)
  (collection_literal)
] @fold

((import_list) @fold.imports
  (#set! collapsed "true"))

(multiline_comment) @fold.comment
(line_comment) @fold.marker
