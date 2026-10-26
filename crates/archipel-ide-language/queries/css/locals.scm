; CSS custom properties: `--name` declared in a block, used through `var(--name)`.
(stylesheet) @local.scope
(block) @local.scope

(declaration (property_name) @local.definition.variable
  (#match? @local.definition.variable "^--"))
(call_expression (function_name) @_f (arguments (plain_value) @local.reference)
  (#eq? @_f "var"))
