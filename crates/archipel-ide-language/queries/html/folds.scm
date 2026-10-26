; HTML folding: elements between their start and end tags, comments.
(element (start_tag) @fold.open (end_tag) @fold.close)
(script_element (start_tag) @fold.open (end_tag) @fold.close)
(style_element (start_tag) @fold.open (end_tag) @fold.close)
(comment) @fold.comment
