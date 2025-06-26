; Comments
(comment) @comment

(jet) @function.call
"jet" @keyword
(fold) @function.call
(for_while) @function.call
(type_cast) @function.call

; Functions and calls
(function_name) @function

; Types
"bool" @type.builtin
(unsigned_type) @type.builtin
(builtin_alias) @type.builtin
(sum_type) @type
(option_type) @type
(tuple_type) @type
(array_type) @type
(list_type) @type
(alias_name) @type

(some_pattern) @type
(left_pattern) @type
(right_pattern) @type

; Keywords
[
  "fn"
  "let"
  "type"
  "mod"
  "const"
  "match"
  "unwrap_left"
  "unwrap_right"
  "is_none"
  "unwrap"
] @keyword

[
  "assert!"
  "panic!"
  "dbg!"
] @preproc

; ; Booleans and special literals
[
  (true_expr)
  (false_expr)
  "true"
  "false"
] @boolean

; Variables
(identifier) @variable
(witness_name) @variable.special

; ; Literals
(bin_literal) @number
(hex_literal) @number
(dec_literal) @number

; ; Witness and param special expressions
(witness_expr) @variable.special
(param_expr) @variable.special

; ; Operators
[
  "=>"
  "="
  "->"
] @operator

; ; Punctuation
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<"
  ">"
] @punctuation.bracket

[
  ","
  ";"
  ":"
  "::"
] @punctuation.delimiter
