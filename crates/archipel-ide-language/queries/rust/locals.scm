; Rust scopes, definitions and references (file-local resolution, tier G1).
(source_file) @local.scope
(function_item) @local.scope
(closure_expression) @local.scope
(block) @local.scope
(for_expression) @local.scope
(match_arm) @local.scope
(impl_item) @local.scope
(trait_item) @local.scope
(mod_item) @local.scope

(function_item name: (identifier) @local.definition.function
  (#set! scope "parent"))
(function_signature_item name: (identifier) @local.definition.function
  (#set! scope "parent"))
(const_item name: (identifier) @local.definition.constant
  (#set! scope "parent"))
(static_item name: (identifier) @local.definition.constant
  (#set! scope "parent"))
(let_declaration pattern: (identifier) @local.definition.variable)
(let_declaration pattern: (mut_pattern (identifier) @local.definition.variable))
(let_declaration pattern: (tuple_pattern (identifier) @local.definition.variable))
(parameter pattern: (identifier) @local.definition.parameter)
(parameter pattern: (mut_pattern (identifier) @local.definition.parameter))
(self_parameter (self) @local.definition.parameter)
(closure_parameters (identifier) @local.definition.parameter)
(closure_parameters (parameter pattern: (identifier) @local.definition.parameter))
(for_expression pattern: (identifier) @local.definition.variable)
(tuple_struct_pattern (identifier) @local.definition.variable)
(use_declaration argument: (scoped_identifier name: (identifier) @local.definition.import))
(use_declaration argument: (identifier) @local.definition.import)
(use_list (identifier) @local.definition.import)
(use_list (scoped_identifier name: (identifier) @local.definition.import))
(use_as_clause alias: (identifier) @local.definition.import)

(identifier) @local.reference
(self) @local.reference
(shorthand_field_initializer (identifier) @local.reference)
