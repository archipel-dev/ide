; Java structure: declarations with their modifiers and details.
(class_declaration (modifiers)? @modifier name: (identifier) @name) @definition.class
(interface_declaration (modifiers)? @modifier name: (identifier) @name) @definition.interface
(annotation_type_declaration (modifiers)? @modifier name: (identifier) @name) @definition.interface
(enum_declaration (modifiers)? @modifier name: (identifier) @name) @definition.enum
(record_declaration (modifiers)? @modifier name: (identifier) @name) @definition.class
(constructor_declaration (modifiers)? @modifier name: (identifier) @name) @definition.constructor
(method_declaration (modifiers)? @modifier name: (identifier) @name) @definition.method
(field_declaration (modifiers)? @modifier
  type: (_) @detail
  declarator: (variable_declarator name: (identifier) @name)
  (#set! detail.prefix ": ")) @definition.field
(enum_constant name: (identifier) @name) @definition.constant
