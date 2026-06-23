; Hash Visualiser (.hv) syntax highlighting.
; Specific patterns come first; the generic `(identifier) @variable` at the end is the fallback.

; - - - definitions & named references - - -
(hash_block name: (identifier) @type)
(function_definition name: (identifier) @function)
(call_expression function: (identifier) @function)
(reduction operator: _ @function.builtin)

(data_declaration name: (identifier) @constant)
(node_declaration name: (identifier) @variable)
(parameter name: (identifier) @variable.parameter)

(property key: (identifier) @property)
(context_setting key: (identifier) @property)
(prop_assignment key: (identifier) @property)
(fill_pulse prop: (identifier) @property)
(prop_transition prop: (identifier) @property)

(event_handler event: (identifier) @function)
(emit_effect event: (identifier) @function)
(wire_declaration wire_name: (identifier) @label)
(comprehension variable: (identifier) @variable.parameter)

; - - - types - - -
(node_kind) @type
(primitive_type) @type.builtin

; - - - constant-like modes - - -
(flow_direction) @constant.builtin
(arrange_mode) @constant.builtin
"all" @constant.builtin

; - - - keywords - - -
[
  "hash"
  "context"
  "fn"
  "node"
  "wire"
  "data"
  "group"
  "layout"
  "import"
  "on"
  "emit"
  "set"
  "let"
  "animate"
  "reroute"
  "contains"
  "arrange"
  "reduce"
  "for"
  "in"
  "over"
  "via"
  "to"
  "from"
] @keyword

; - - - operators - - -
[
  "and"
  "or"
  "xor"
  "not"
  "shl"
  "shr_u"
  "shr_s"
  "rotr_u"
  "rotr_s"
  "rotl_u"
  "rotl_s"
  "+"
  "-"
  "->"
  "=>"
  ".."
  "="
] @operator

; - - - punctuation - - -
[ "(" ")" "[" "]" "{" "}" ] @punctuation.bracket
[ "," ";" ":" ] @punctuation.delimiter
"?" @punctuation.special

; - - - literals - - -
(comment) @comment
(string) @string
(triple_string) @string
(integer) @number
(hex_literal) @number
(duration) @number

; - - - fallback - - -
(identifier) @variable
