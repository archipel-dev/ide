; Java scopes, definitions and references (file-local resolution, tier G1).
(program) @local.scope
(class_declaration) @local.scope
(interface_declaration) @local.scope
(enum_declaration) @local.scope
(record_declaration) @local.scope
(method_declaration) @local.scope
(constructor_declaration) @local.scope
(lambda_expression) @local.scope
(block) @local.scope
(for_statement) @local.scope
(enhanced_for_statement) @local.scope
(catch_clause) @local.scope
(switch_block_statement_group) @local.scope
(try_with_resources_statement) @local.scope

(method_declaration name: (identifier) @local.definition.method
  (#set! scope "parent"))
(class_declaration name: (identifier) @local.definition.class
  (#set! scope "parent"))
(interface_declaration name: (identifier) @local.definition.class
  (#set! scope "parent"))
(enum_declaration name: (identifier) @local.definition.class
  (#set! scope "parent"))
(record_declaration name: (identifier) @local.definition.class
  (#set! scope "parent"))
(enum_constant name: (identifier) @local.definition.constant)
(field_declaration declarator: (variable_declarator name: (identifier) @local.definition.field))
(local_variable_declaration declarator: (variable_declarator name: (identifier) @local.definition.variable))
(formal_parameter name: (identifier) @local.definition.parameter)
(spread_parameter (variable_declarator name: (identifier) @local.definition.parameter))
(catch_formal_parameter name: (identifier) @local.definition.parameter)
(enhanced_for_statement name: (identifier) @local.definition.variable)
(lambda_expression parameters: (identifier) @local.definition.parameter)
(inferred_parameters (identifier) @local.definition.parameter)
(resource name: (identifier) @local.definition.variable)
(import_declaration (scoped_identifier name: (identifier) @local.definition.import))

(identifier) @local.reference
