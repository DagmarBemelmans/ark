; Copied from ocsmit/zed-r (https://github.com/ocsmit/zed-r), commit 87438f0.
; Copyright (c) 2026 Owen Smith and zed-r contributors. MIT License.
; See editors/zed/README.md for the full attribution and license notice.

; [
;   (brace_list)
;   (paren_list)
;   (special)
;   (pipe)
;   (call)
;   "|>"
;   "if"
;   "else"
;   "while"
;   "repeat"
;   "for"
; ] @indent.begin

; (binary
;   operator: (special)) @indent.begin

; [
;   "}"
;   ")"
; ] @indent.branch

; ((formal_parameters
;   (identifier)) @indent.align
;   (#set! indent.open_delimiter "(")
;   (#set! indent.close_delimiter ")"))

; [
;   ")"
;   "}"
; ] @indent.end

; (comment) @indent.ignore

; keep the same level of indentation

(_ "[" "]" @end) @indent
(_ "{" "}" @end) @indent
(_ "(" ")" @end) @indent
