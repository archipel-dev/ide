; TypeScript structure: classes, interfaces, enums, functions, methods, fields.
(class_declaration name: (type_identifier) @name) @definition.class
(abstract_class_declaration name: (type_identifier) @name) @definition.class
(interface_declaration name: (type_identifier) @name) @definition.interface
(enum_declaration name: (identifier) @name) @definition.enum
(type_alias_declaration name: (type_identifier) @name) @definition.type
(internal_module name: (_) @name) @definition.module
(function_declaration ["async"]* @modifier name: (identifier) @name) @definition.function
(class_body
  (method_definition (accessibility_modifier)? @modifier ["static" "async" "get" "set"]* @modifier name: (_) @name) @definition.method)
(class_body
  (public_field_definition (accessibility_modifier)? @modifier ["static" "readonly"]* @modifier name: (_) @name type: (type_annotation)? @detail
    (#set! detail.prefix "")) @definition.field)
(interface_body
  (method_signature name: (_) @name) @definition.method)
(interface_body
  (property_signature name: (_) @name type: (type_annotation)? @detail) @definition.field)
(enum_body name: (property_identifier) @name) @definition.constant
(program
  (lexical_declaration
    (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression)])) @definition.function)
(export_statement
  declaration: (lexical_declaration
    (variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression)])) @definition.function)
