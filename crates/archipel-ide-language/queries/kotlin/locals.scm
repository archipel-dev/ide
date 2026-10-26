; Kotlin scopes, definitions and references (file-local resolution, tier G1).
(source_file) @local.scope
(class_declaration) @local.scope
(object_declaration) @local.scope
(function_declaration) @local.scope
(anonymous_function) @local.scope
(lambda_literal) @local.scope
(for_statement) @local.scope
(control_structure_body) @local.scope
(when_entry) @local.scope
(catch_block) @local.scope

(function_declaration (simple_identifier) @local.definition.function
  (#set! scope "parent"))
(class_declaration (type_identifier) @local.definition.class
  (#set! scope "parent"))
(object_declaration (type_identifier) @local.definition.class
  (#set! scope "parent"))
(property_declaration (variable_declaration (simple_identifier) @local.definition.variable))
(property_declaration (multi_variable_declaration (variable_declaration (simple_identifier) @local.definition.variable)))
(parameter (simple_identifier) @local.definition.parameter)
(class_parameter (simple_identifier) @local.definition.field)
(for_statement (variable_declaration (simple_identifier) @local.definition.variable))
(lambda_parameters (variable_declaration (simple_identifier) @local.definition.parameter))
(catch_block (simple_identifier) @local.definition.parameter)
(enum_entry (simple_identifier) @local.definition.constant)
(import_header (identifier (simple_identifier) @local.definition.import .))

(simple_identifier) @local.reference
