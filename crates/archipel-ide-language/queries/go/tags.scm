; Go structure: types, functions, methods, fields and constants.
(type_declaration (type_spec name: (type_identifier) @name type: (struct_type))) @definition.class
(type_declaration (type_spec name: (type_identifier) @name type: (interface_type))) @definition.interface
(type_declaration (type_spec name: (type_identifier) @name)) @definition.type
(function_declaration name: (identifier) @name) @definition.function
(method_declaration name: (field_identifier) @name) @definition.method
(method_elem name: (field_identifier) @name) @definition.method
(field_declaration name: (field_identifier) @name type: (_) @detail
  (#set! detail.prefix ": ")) @definition.field
(const_spec name: (identifier) @name) @definition.constant
