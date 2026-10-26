; Shell selection groups and text objects.
[
  (compound_statement)
  (do_group)
  (case_statement)
  (program)
] @select.group

(function_definition) @function.outer
(function_definition body: (compound_statement) @function.inner)
(do_group) @block.outer
(comment) @comment.outer
