; JavaScript pairs: classify code blocks and JSX tags.
(statement_block "{" @open.block "}" @close.block)
(class_body "{" @open.block "}" @close.block)
(switch_body "{" @open.block "}" @close.block)
(object "{" @open.brace "}" @close.brace)
(jsx_opening_element) @open.tag
(jsx_closing_element) @close.tag
