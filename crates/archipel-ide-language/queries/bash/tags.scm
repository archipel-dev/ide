; Shell structure: functions and top-level variables.
(function_definition name: (word) @name) @definition.function
(program (variable_assignment name: (variable_name) @name) @definition.constant)
(program (declaration_command (variable_assignment name: (variable_name) @name)) @definition.constant)
