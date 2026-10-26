; Archipel IDE structure query: headings, nested by tree-sitter-md's sections.
(section
  (atx_heading
    heading_content: (inline) @name)) @definition.heading

(section
  (setext_heading
    heading_content: (paragraph) @name)) @definition.heading
