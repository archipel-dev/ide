; Kotlin pairs: classify code blocks and add generic angle brackets.
(class_body "{" @open.block "}" @close.block)
(enum_class_body "{" @open.block "}" @close.block)
(function_body "{" @open.block "}" @close.block)
(control_structure_body "{" @open.block "}" @close.block)
(lambda_literal "{" @open.block "}" @close.block)
(when_expression "{" @open.block "}" @close.block)
(type_arguments "<" @open.angle ">" @close.angle)
(type_parameters "<" @open.angle ">" @close.angle)
