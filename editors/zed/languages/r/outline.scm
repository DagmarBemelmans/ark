; Copied from ocsmit/zed-r (https://github.com/ocsmit/zed-r), commit 87438f0.
; Copyright (c) 2026 Owen Smith and zed-r contributors. MIT License.
; See editors/zed/README.md for the full attribution and license notice.

; Variables by assignment operators
(binary_operator
    lhs: (identifier) @name
    operator: ["<-" "=" "<<-"]
    rhs: (_)
) @item

(binary_operator
    lhs: (_)
    operator: ["->" "->>"]
    rhs: (identifier) @name
) @item

; Variables by `for` loop
(for_statement
  "for" @context
  "(" @context
  variable: (identifier) @name
  "in" @context
  sequence: (_) @context
  ")" @context
) @item

; Jupyter cell tag
(
    (comment) @name
    (#match? @name "^#\\s?%%")
) @item
