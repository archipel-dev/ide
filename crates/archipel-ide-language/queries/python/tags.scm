; Python structure: classes, functions, methods and class attributes.
(class_definition name: (identifier) @name) @definition.class
(class_definition
  body: (block
    (function_definition ["async"]* @modifier name: (identifier) @name) @definition.method))
(class_definition
  body: (block
    (decorated_definition
      definition: (function_definition ["async"]* @modifier name: (identifier) @name) @definition.method)))
(function_definition ["async"]* @modifier name: (identifier) @name) @definition.function
(module (expression_statement (assignment left: (identifier) @name)) @definition.constant)
(class_definition
  body: (block (expression_statement (assignment left: (identifier) @name)) @definition.field))
