; Rust pairs. `(`, `[` and `{` always pair; this file classifies the code
; blocks and adds the angle brackets of generics (never comparison operators).
(block "{" @open.block "}" @close.block)
(declaration_list "{" @open.block "}" @close.block)
(field_declaration_list "{" @open.block "}" @close.block)
(enum_variant_list "{" @open.block "}" @close.block)
(match_block "{" @open.block "}" @close.block)
(type_arguments "<" @open.angle ">" @close.angle)
(type_parameters "<" @open.angle ">" @close.angle)
(bracketed_type "<" @open.angle ">" @close.angle)
