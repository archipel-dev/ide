; Python documents a definition with the first string of its body.
(function_definition body: (block . (expression_statement (string) @doc.body)))
(class_definition body: (block . (expression_statement (string) @doc.body)))
(decorator) @doc.skip
