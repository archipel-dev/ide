; Go scopes, definitions and references (file-local resolution, tier G1).
(source_file) @local.scope
(function_declaration) @local.scope
(method_declaration) @local.scope
(func_literal) @local.scope
(block) @local.scope
(for_statement) @local.scope
(expression_case) @local.scope
(default_case) @local.scope

(function_declaration name: (identifier) @local.definition.function
  (#set! scope "parent"))
(parameter_declaration name: (identifier) @local.definition.parameter)
(variadic_parameter_declaration name: (identifier) @local.definition.parameter)
(short_var_declaration left: (expression_list (identifier) @local.definition.variable))
(var_spec name: (identifier) @local.definition.variable)
(const_spec name: (identifier) @local.definition.constant)
(range_clause left: (expression_list (identifier) @local.definition.variable))
(type_spec name: (type_identifier) @local.definition.class)
(import_spec name: (package_identifier) @local.definition.import)

(identifier) @local.reference
