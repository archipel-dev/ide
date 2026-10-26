; JavaScript structure: classes, functions, methods and fields.
(class_declaration name: (identifier) @name) @definition.class
(function_declaration ["async"]* @modifier name: (identifier) @name) @definition.function
(generator_function_declaration name: (identifier) @name) @definition.function
(class_body
  (method_definition ["static" "async" "get" "set"]* @modifier name: (_) @name) @definition.method)
(class_body
  (field_definition ["static"]* @modifier property: (_) @name) @definition.field)
(program
  (lexical_declaration
    (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression)])) @definition.function)
(export_statement
  declaration: (lexical_declaration
    (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression)])) @definition.function)
(program
  (lexical_declaration
    (variable_declarator name: (identifier) @name value: [(number) (string) (object) (array)])) @definition.constant)
