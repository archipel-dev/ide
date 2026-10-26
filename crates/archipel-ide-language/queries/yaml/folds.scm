; YAML folding: nested mappings and sequences fold after their key line.
(block_mapping_pair value: (block_node) @fold)
(block_sequence_item (block_node) @fold)
(flow_mapping) @fold
(flow_sequence) @fold
(comment) @fold.marker
