; Starlark structure: functions, rule/macro assignments and target calls by name.
(function_definition name: (identifier) @name) @definition.function
(module (expression_statement (assignment left: (identifier) @name)) @definition.constant)
(module
  (expression_statement
    (call
      arguments: (argument_list
        (keyword_argument name: (identifier) @_k value: (string (string_content) @name)))
      (#eq? @_k "name"))) @definition.module)
