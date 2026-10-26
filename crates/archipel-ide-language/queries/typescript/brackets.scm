; TypeScript pairs: classify code blocks and add generic angle brackets.
(statement_block "{" @open.block "}" @close.block)
(class_body "{" @open.block "}" @close.block)
(interface_body "{" @open.block "}" @close.block)
(enum_body "{" @open.block "}" @close.block)
(switch_body "{" @open.block "}" @close.block)
(object "{" @open.brace "}" @close.brace)
(type_arguments "<" @open.angle ">" @close.angle)
(type_parameters "<" @open.angle ">" @close.angle)
