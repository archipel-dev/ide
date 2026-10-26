; Shell indentation: then/do/in bodies, `fi`/`done`/`esac` align with the opener.
[
  (compound_statement)
  (if_statement)
  (elif_clause)
  (else_clause)
  (do_group)
  (case_statement)
  (case_item)
  (subshell)
] @indent

["}" ")" "fi" "done" "esac"] @outdent

[(elif_clause) (else_clause)] @indent.branch

["{" "(" "then" "do" "in" "else"] @indent.after
(case_item ")" @indent.after)
