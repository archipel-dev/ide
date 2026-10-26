; Go pairs: classify code blocks.
(block "{" @open.block "}" @close.block)
(field_declaration_list "{" @open.block "}" @close.block)
(literal_value "{" @open.brace "}" @close.brace)
(type_parameter_list "[" @open.bracket "]" @close.bracket)
