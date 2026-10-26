; Shell scopes, definitions and references (file-local resolution, tier G1).
(program) @local.scope
(function_definition) @local.scope

(function_definition name: (word) @local.definition.function
  (#set! scope "parent"))
(variable_assignment name: (variable_name) @local.definition.variable)
(for_statement variable: (variable_name) @local.definition.variable)
(command name: (command_name (word) @_c) argument: (word) @local.definition.variable
  (#any-of? @_c "read" "mapfile" "readarray"))

(variable_name) @local.reference
(command name: (command_name (word) @local.reference))
