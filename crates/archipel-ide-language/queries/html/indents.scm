; HTML indentation: children sit one level inside their element.
[
  (element)
  (script_element)
  (style_element)
] @indent

(end_tag) @outdent
(start_tag ">" @indent.after)
