; Groovy structure: declarations with their modifiers.
(class_declaration ["public" "private" "protected" "static" "abstract" "final"]* @modifier name: (identifier) @name) @definition.class
(interface_declaration name: (identifier) @name) @definition.interface
(enum_declaration name: (identifier) @name) @definition.enum
(constructor_declaration ["public" "private" "protected"]* @modifier name: (identifier) @name) @definition.constructor
(method_declaration ["public" "private" "protected" "static" "abstract" "final"]* @modifier name: (identifier) @name) @definition.method
(field_declaration ["public" "private" "protected" "static" "final"]* @modifier
  type: (_) @detail
  (variable_declarator name: (identifier) @name)
  (#set! detail.prefix ": ")) @definition.field
