; C scopes, definitions and references (file-local resolution, tier G1).
(translation_unit) @local.scope
(function_definition) @local.scope
(compound_statement) @local.scope
(for_statement) @local.scope

(function_declarator declarator: (identifier) @local.definition.function
  (#set! scope "parent"))
(parameter_declaration declarator: (identifier) @local.definition.parameter)
(parameter_declaration declarator: (pointer_declarator declarator: (identifier) @local.definition.parameter))
(parameter_declaration declarator: (pointer_declarator declarator: (pointer_declarator declarator: (identifier) @local.definition.parameter)))
(parameter_declaration declarator: (array_declarator declarator: (identifier) @local.definition.parameter))
(declaration declarator: (identifier) @local.definition.variable)
(declaration declarator: (init_declarator declarator: (identifier) @local.definition.variable))
(declaration declarator: (init_declarator declarator: (pointer_declarator declarator: (identifier) @local.definition.variable)))
(declaration declarator: (pointer_declarator declarator: (identifier) @local.definition.variable))
(declaration declarator: (array_declarator declarator: (identifier) @local.definition.variable))
(enumerator name: (identifier) @local.definition.constant)
(preproc_def name: (identifier) @local.definition.constant)
(preproc_function_def name: (identifier) @local.definition.function)
(type_definition declarator: (type_identifier) @local.definition.class)
(struct_specifier name: (type_identifier) @local.definition.class body: (_))

(identifier) @local.reference
