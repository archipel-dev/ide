; Groovy scopes, definitions and references (file-local resolution, tier G1).
(source_file) @local.scope
(class_declaration) @local.scope
(method_declaration) @local.scope
(constructor_declaration) @local.scope
(closure) @local.scope
(block) @local.scope
(for_statement) @local.scope

(method_declaration name: (identifier) @local.definition.method
  (#set! scope "parent"))
(class_declaration name: (identifier) @local.definition.class
  (#set! scope "parent"))
(field_declaration (variable_declarator name: (identifier) @local.definition.field))
(local_variable_declaration (variable_declarator name: (identifier) @local.definition.variable))
(local_variable_declaration name: (identifier) @local.definition.variable)
(formal_parameter name: (identifier) @local.definition.parameter)
(closure_parameter name: (identifier) @local.definition.parameter)

(identifier) @local.reference
