; Groovy selection groups and text objects.
[
  (argument_list)
  (formal_parameters)
  (map_literal)
  (list_literal)
  (block)
  (class_body)
  (closure)
  (enum_body)
] @select.group

(method_declaration) @function.outer
(constructor_declaration) @function.outer
(closure) @function.outer
(class_declaration) @class.outer
(interface_declaration) @class.outer
(enum_declaration) @class.outer
(formal_parameters) @parameter.list
(block_comment) @comment.outer
(groovydoc_comment) @comment.outer
(line_comment) @comment.outer
