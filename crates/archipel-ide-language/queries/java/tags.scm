; Archipel IDE extensions appended after the upstream tree-sitter-java tags query,
; covering declarations IntelliJ's Structure view lists that upstream omits.

(constructor_declaration
  name: (identifier) @name) @definition.constructor

(field_declaration
  declarator: (variable_declarator
    name: (identifier) @name)) @definition.field

(enum_declaration
  name: (identifier) @name) @definition.enum

(enum_constant
  name: (identifier) @name) @definition.constant

(record_declaration
  name: (identifier) @name) @definition.class

(annotation_type_declaration
  name: (identifier) @name) @definition.interface
