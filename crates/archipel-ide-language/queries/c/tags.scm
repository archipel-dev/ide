; C structure: types, functions, fields, enumerators, macros.
(struct_specifier name: (type_identifier) @name body: (_)) @definition.class
(union_specifier name: (type_identifier) @name body: (_)) @definition.class
(enum_specifier name: (type_identifier) @name body: (_)) @definition.enum
(type_definition declarator: (type_identifier) @name) @definition.type
(function_definition (storage_class_specifier)? @modifier declarator: (function_declarator declarator: (identifier) @name)) @definition.function
(function_definition (storage_class_specifier)? @modifier declarator: (pointer_declarator declarator: (function_declarator declarator: (identifier) @name))) @definition.function
(field_declaration type: (_) @detail declarator: (field_identifier) @name
  (#set! detail.prefix ": ")) @definition.field
(enumerator name: (identifier) @name) @definition.constant
(preproc_def name: (identifier) @name) @definition.macro
(preproc_function_def name: (identifier) @name) @definition.macro
