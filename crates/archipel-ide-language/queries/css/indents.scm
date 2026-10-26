; CSS indentation: declarations sit one level inside their block.
[
  (block)
  (keyframe_block_list)
  (arguments)
] @indent

["}" ")"] @outdent
["{" "("] @indent.after
