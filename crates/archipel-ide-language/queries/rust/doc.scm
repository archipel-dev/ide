; Documentation comments and the siblings that may sit between a doc comment
; and the declaration it documents.
((line_comment (outer_doc_comment_marker)) @doc)
((block_comment (outer_doc_comment_marker)) @doc)
((line_comment (inner_doc_comment_marker)) @doc.inner)
(attribute_item) @doc.skip
