; C++ structure: namespaces, classes, functions, methods and fields.
(namespace_definition name: (namespace_identifier) @name) @definition.module
(class_specifier name: (type_identifier) @name body: (_)) @definition.class
(struct_specifier name: (type_identifier) @name body: (_)) @definition.class
(union_specifier name: (type_identifier) @name body: (_)) @definition.class
(enum_specifier name: (type_identifier) @name body: (_)) @definition.enum
(field_declaration_list
  (function_definition declarator: (function_declarator declarator: (field_identifier) @name)) @definition.method)
(field_declaration_list
  (function_definition declarator: (function_declarator declarator: (identifier) @name)) @definition.constructor)
(field_declaration_list
  (function_definition declarator: (function_declarator declarator: (destructor_name) @name)) @definition.method)
(field_declaration_list
  (field_declaration declarator: (function_declarator declarator: (field_identifier) @name)) @definition.method)
(function_definition declarator: (function_declarator declarator: (qualified_identifier name: (identifier) @name))) @definition.method
(function_definition declarator: (function_declarator declarator: (identifier) @name)) @definition.function
(field_declaration type: (_) @detail declarator: (field_identifier) @name
  (#set! detail.prefix ": ")) @definition.field
(enumerator name: (identifier) @name) @definition.constant
(type_definition declarator: (type_identifier) @name) @definition.type
(alias_declaration name: (type_identifier) @name) @definition.type
(preproc_def name: (identifier) @name) @definition.macro
