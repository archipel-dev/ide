; C++ scopes, definitions and references (file-local resolution, tier G1).
(translation_unit) @local.scope
(function_definition) @local.scope
(lambda_expression) @local.scope
(compound_statement) @local.scope
(for_statement) @local.scope
(for_range_loop) @local.scope
(class_specifier) @local.scope
(namespace_definition) @local.scope

(function_declarator declarator: (identifier) @local.definition.function
  (#set! scope "parent"))
(function_declarator declarator: (field_identifier) @local.definition.method
  (#set! scope "parent"))
(parameter_declaration declarator: (identifier) @local.definition.parameter)
(parameter_declaration declarator: (reference_declarator (identifier) @local.definition.parameter))
(parameter_declaration declarator: (pointer_declarator declarator: (identifier) @local.definition.parameter))
(optional_parameter_declaration declarator: (identifier) @local.definition.parameter)
(declaration declarator: (identifier) @local.definition.variable)
(declaration declarator: (init_declarator declarator: (identifier) @local.definition.variable))
(declaration declarator: (pointer_declarator declarator: (identifier) @local.definition.variable))
(for_range_loop declarator: (identifier) @local.definition.variable)
(field_declaration declarator: (field_identifier) @local.definition.field)
(enumerator name: (identifier) @local.definition.constant)
(class_specifier name: (type_identifier) @local.definition.class)
(struct_specifier name: (type_identifier) @local.definition.class body: (_))
(namespace_definition name: (namespace_identifier) @local.definition.module)

(identifier) @local.reference
