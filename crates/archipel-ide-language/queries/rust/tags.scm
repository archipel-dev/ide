; Rust structure: declarations with their modifiers and details.
(struct_item name: (type_identifier) @name) @definition.class
(struct_item (visibility_modifier) @modifier name: (type_identifier) @name) @definition.class
(union_item name: (type_identifier) @name) @definition.class
(enum_item name: (type_identifier) @name) @definition.enum
(enum_item (visibility_modifier) @modifier name: (type_identifier) @name) @definition.enum
(trait_item name: (type_identifier) @name) @definition.interface
(trait_item (visibility_modifier) @modifier name: (type_identifier) @name) @definition.interface
(type_item name: (type_identifier) @name) @definition.type
(mod_item name: (identifier) @name) @definition.module
(mod_item (visibility_modifier) @modifier name: (identifier) @name) @definition.module
(macro_definition name: (identifier) @name) @definition.macro

(impl_item type: (_) @name trait: (_)? @detail
  (#set! detail.prefix ": ")) @definition.implementation

(declaration_list
  (function_item (visibility_modifier)? @modifier (function_modifiers)? @modifier name: (identifier) @name) @definition.method)
(declaration_list
  (function_signature_item name: (identifier) @name) @definition.method)
(function_item (visibility_modifier)? @modifier (function_modifiers)? @modifier name: (identifier) @name) @definition.function

(const_item (visibility_modifier)? @modifier name: (identifier) @name type: (_) @detail
  (#set! detail.prefix ": ")) @definition.constant
(static_item (visibility_modifier)? @modifier name: (identifier) @name type: (_) @detail
  (#set! detail.prefix ": ")) @definition.constant
(field_declaration (visibility_modifier)? @modifier name: (field_identifier) @name type: (_) @detail
  (#set! detail.prefix ": ")) @definition.field
(enum_variant name: (identifier) @name) @definition.constant
