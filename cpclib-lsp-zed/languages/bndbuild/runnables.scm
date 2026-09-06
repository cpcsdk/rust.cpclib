; Runnable queries for bndbuild YAML files
; Match target definitions: any YAML mapping with targets:/tgt:/target:/build: keys
; The @run capture marks where the run button appears
; The @target capture becomes ZED_CUSTOM_target environment variable

; Match block_mapping_pair with targets/tgt/target/build key
; Captures the target value (file path/name) for command execution
(block_mapping_pair
  key: (flow_node) @_key
  (#match? @_key "^(targets|tgt|target|build)$")
  value: (flow_node) @run @target
  (#set! tag bndbuild-target))

; Match block mappings in sequences (arrays of rules)
(block_sequence_item
  (block_node
    (block_mapping
      (block_mapping_pair
        key: (flow_node) @_key
        (#match? @_key "^(targets|tgt|target|build)$")
        value: (flow_node) @run @target
        (#set! tag bndbuild-target)))))
