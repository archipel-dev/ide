; Python scopes, definitions and references (file-local resolution, tier G1).
(module) @local.scope
(function_definition) @local.scope
(class_definition) @local.scope
(lambda) @local.scope
(list_comprehension) @local.scope
(dictionary_comprehension) @local.scope
(set_comprehension) @local.scope
(generator_expression) @local.scope

(function_definition name: (identifier) @local.definition.function
  (#set! scope "parent"))
(class_definition name: (identifier) @local.definition.class
  (#set! scope "parent"))
(parameters (identifier) @local.definition.parameter)
(parameters (default_parameter name: (identifier) @local.definition.parameter))
(parameters (typed_parameter (identifier) @local.definition.parameter))
(parameters (typed_default_parameter name: (identifier) @local.definition.parameter))
(parameters (list_splat_pattern (identifier) @local.definition.parameter))
(parameters (dictionary_splat_pattern (identifier) @local.definition.parameter))
(lambda_parameters (identifier) @local.definition.parameter)
(assignment left: (identifier) @local.definition.variable)
(assignment left: (pattern_list (identifier) @local.definition.variable))
(assignment left: (tuple_pattern (identifier) @local.definition.variable))
(augmented_assignment left: (identifier) @local.definition.variable)
(for_statement left: (identifier) @local.definition.variable)
(for_statement left: (pattern_list (identifier) @local.definition.variable))
(for_in_clause left: (identifier) @local.definition.variable)
(for_in_clause left: (pattern_list (identifier) @local.definition.variable))
(as_pattern_target (identifier) @local.definition.variable)
(named_expression name: (identifier) @local.definition.variable)
(import_statement name: (dotted_name . (identifier) @local.definition.import))
(import_statement name: (aliased_import alias: (identifier) @local.definition.import))
(import_from_statement name: (dotted_name (identifier) @local.definition.import))
(import_from_statement name: (aliased_import alias: (identifier) @local.definition.import))

(identifier) @local.reference
