; C pairs: classify code blocks.
(compound_statement "{" @open.block "}" @close.block)
(field_declaration_list "{" @open.block "}" @close.block)
(enumerator_list "{" @open.block "}" @close.block)
(initializer_list "{" @open.brace "}" @close.brace)
