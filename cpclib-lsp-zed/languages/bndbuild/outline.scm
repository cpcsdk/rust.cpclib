;; Outline query for bndbuild files
;; Captures build targets (and their values) for the document outline
;; LSP provides the full hierarchical outline (Variables + Artifacts)
;; This tree-sitter query is a fallback when LSP is not available

;; Build targets (all synonyms: targets, tgt, target, build)
;; @name captures the target value (filename/path) to display in outline
;; @item captures the entire mapping pair for navigation
(block_mapping_pair
  key: (flow_node) @_key
  (#match? @_key "^(targets|tgt|target|build)$")
  value: (flow_node
    (plain_scalar
      (string_scalar) @name)) @item)

;; Match targets in flow style: targets: [file.bin]
(block_mapping_pair
  key: (flow_node) @_key
  (#match? @_key "^(targets|tgt|target|build)$")
  value: (flow_node) @name) @item
