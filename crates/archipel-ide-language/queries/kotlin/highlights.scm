; Archipel IDE extensions appended after the upstream tree-sitter-kotlin-sg
; highlights query (later patterns win). Each pattern fills a gap upstream.

; Upstream anchors the name as the first named child, which misses functions
; with modifiers (`override fun`, `private fun`).
(function_declaration
  (simple_identifier) @function)

; Upstream has no capture for the `package` keyword.
(package_header
  "package" @keyword)
