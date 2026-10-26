; Kotlin structure: declarations with their modifiers and details.
(class_declaration (modifiers)? @modifier (type_identifier) @name) @definition.class
(object_declaration (modifiers)? @modifier (type_identifier) @name) @definition.class
(companion_object (type_identifier) @name) @definition.class
(function_declaration (modifiers)? @modifier (simple_identifier) @name) @definition.function
(class_body
  (property_declaration (modifiers)? @modifier
    (variable_declaration (simple_identifier) @name (user_type)? @detail
      (#set! detail.prefix ": "))) @definition.field)
(source_file
  (property_declaration (modifiers)? @modifier
    (variable_declaration (simple_identifier) @name (user_type)? @detail
      (#set! detail.prefix ": "))) @definition.field)
(enum_entry (simple_identifier) @name) @definition.constant
(type_alias (type_identifier) @name) @definition.type
