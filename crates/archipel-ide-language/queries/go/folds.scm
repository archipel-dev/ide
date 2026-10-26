; Go folding: blocks, type bodies, literals, import lists, case clauses, comments.
[
  (block)
  (field_declaration_list)
  (literal_value)
  (import_spec_list)
  (const_declaration)
  (var_declaration)
  (expression_case)
  (default_case)
  (interface_type)
] @fold

((import_declaration) @fold.imports
  (#set! collapsed "true"))

(comment) @fold.comment
(comment) @fold.marker
