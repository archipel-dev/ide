; XML folding: elements between their tags, comments, CDATA.
(element (STag) @fold.open (ETag) @fold.close)
(Comment) @fold.comment
(CDSect) @fold.comment
