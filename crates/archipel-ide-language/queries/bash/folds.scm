; Shell folding: function bodies, then/do/in blocks, here-documents, comment runs.
(compound_statement) @fold
(if_statement "then" @fold.open "fi" @fold.close)
(do_group "do" @fold.open "done" @fold.close)
(case_statement "in" @fold.open "esac" @fold.close)
(heredoc_body) @fold
(comment) @fold.marker
