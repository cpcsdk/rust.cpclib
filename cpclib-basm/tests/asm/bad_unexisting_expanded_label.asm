; Before 20260426:   = error: Unknown symbol: code_{{loop}}
; Since  20260426:   = error: Unknown symbol: code_4

repeat 5, loop
	call code_{{loop}}  ; This should produce an error for unexisting labels without using loop in the error message
endr



code_0
code_1
code_2
code_3