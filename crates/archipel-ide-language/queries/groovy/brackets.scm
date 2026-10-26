; Groovy pairs: classify code blocks.
(block "{" @open.block "}" @close.block)
(class_body "{" @open.block "}" @close.block)
(closure "{" @open.block "}" @close.block)
(enum_body "{" @open.block "}" @close.block)
