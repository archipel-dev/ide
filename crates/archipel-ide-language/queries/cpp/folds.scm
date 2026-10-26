; C++ folding: bodies, namespaces, lambdas, initializers, includes, comments.
[
  (compound_statement)
  (field_declaration_list)
  (enumerator_list)
  (initializer_list)
  (declaration_list)
  (preproc_if)
  (preproc_ifdef)
  (case_statement)
] @fold

((preproc_include) @fold.imports
  (#set! collapsed "true"))

(comment) @fold.comment
(comment) @fold.marker
