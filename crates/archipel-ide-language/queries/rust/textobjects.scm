; Rust selection groups (Extend Selection offers each element, the elements
; up to it, all of them, then the container) and text objects.
[
  (arguments)
  (parameters)
  (array_expression)
  (tuple_expression)
  (block)
  (declaration_list)
  (field_declaration_list)
  (enum_variant_list)
  (match_block)
  (field_initializer_list)
  (type_arguments)
  (closure_parameters)
] @select.group

(function_item) @function.outer
(function_item body: (block) @function.inner)
(function_signature_item) @function.outer
(closure_expression) @function.outer
(struct_item) @class.outer
(enum_item) @class.outer
(trait_item) @class.outer
(impl_item) @class.outer
(mod_item) @class.outer
(parameters) @parameter.list
(block_comment) @comment.outer
(line_comment) @comment.outer
