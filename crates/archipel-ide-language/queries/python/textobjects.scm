; Python selection groups and text objects.
[
  (argument_list)
  (parameters)
  (list)
  (tuple)
  (set)
  (dictionary)
  (block)
  (module)
] @select.group

(function_definition) @function.outer
(function_definition body: (block) @function.inner)
(lambda) @function.outer
(class_definition) @class.outer
(block) @block.outer
(parameters) @parameter.list
(comment) @comment.outer
