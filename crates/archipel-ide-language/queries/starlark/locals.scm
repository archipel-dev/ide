; Starlark scopes, definitions and references (file-local resolution, tier G1).
(module) @local.scope
(function_definition) @local.scope
(lambda) @local.scope
(list_comprehension) @local.scope
(dictionary_comprehension) @local.scope

(function_definition name: (identifier) @local.definition.function
  (#set! scope "parent"))
(parameters (identifier) @local.definition.parameter)
(parameters (default_parameter name: (identifier) @local.definition.parameter))
(parameters (list_splat_pattern (identifier) @local.definition.parameter))
(parameters (dictionary_splat_pattern (identifier) @local.definition.parameter))
(lambda_parameters (identifier) @local.definition.parameter)
(assignment left: (identifier) @local.definition.variable)
(assignment left: (pattern_list (identifier) @local.definition.variable))
(augmented_assignment left: (identifier) @local.definition.variable)
(for_statement left: (identifier) @local.definition.variable)
(for_in_clause left: (identifier) @local.definition.variable)
(call function: (identifier) @_f arguments: (argument_list (string (string_content) @local.definition.import))
  (#eq? @_f "load"))

(identifier) @local.reference
