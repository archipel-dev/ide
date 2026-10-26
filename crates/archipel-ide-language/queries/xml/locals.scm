; XML ids (`xml:id`, `id`) are file-local definitions.
(document) @local.scope

(Attribute (Name) @_n (AttValue) @local.definition.id
  (#match? @_n "^(xml:)?id$"))
