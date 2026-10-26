; C++ pairs: classify code blocks and add template angle brackets.
(compound_statement "{" @open.block "}" @close.block)
(field_declaration_list "{" @open.block "}" @close.block)
(enumerator_list "{" @open.block "}" @close.block)
(declaration_list "{" @open.block "}" @close.block)
(initializer_list "{" @open.brace "}" @close.brace)
(template_argument_list "<" @open.angle ">" @close.angle)
(template_parameter_list "<" @open.angle ">" @close.angle)
