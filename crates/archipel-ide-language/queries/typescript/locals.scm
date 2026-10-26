; JavaScript scopes, definitions and references (file-local resolution, tier G1).
(program) @local.scope
(function_declaration) @local.scope
(function_expression) @local.scope
(arrow_function) @local.scope
(method_definition) @local.scope
(class_declaration) @local.scope
(statement_block) @local.scope
(for_statement) @local.scope
(for_in_statement) @local.scope
(catch_clause) @local.scope

(function_declaration name: (identifier) @local.definition.function
  (#set! scope "parent"))
(class_declaration name: (type_identifier) @local.definition.class
  (#set! scope "parent"))
(arrow_function parameter: (identifier) @local.definition.parameter)
(variable_declarator name: (identifier) @local.definition.variable)
(variable_declarator name: (object_pattern (shorthand_property_identifier_pattern) @local.definition.variable))
(variable_declarator name: (array_pattern (identifier) @local.definition.variable))
(for_in_statement left: (identifier) @local.definition.variable)
(catch_clause parameter: (identifier) @local.definition.parameter)
(import_clause (identifier) @local.definition.import)
(import_specifier name: (identifier) @local.definition.import)
(namespace_import (identifier) @local.definition.import)

(identifier) @local.reference
(shorthand_property_identifier) @local.reference
(required_parameter pattern: (identifier) @local.definition.parameter)
(optional_parameter pattern: (identifier) @local.definition.parameter)
(interface_declaration name: (type_identifier) @local.definition.class)
(type_alias_declaration name: (type_identifier) @local.definition.class)
(enum_declaration name: (identifier) @local.definition.class)
