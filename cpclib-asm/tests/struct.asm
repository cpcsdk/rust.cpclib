; structure st1 created with two fields ch1 and ch2.

	struct st1
ch1 defw 0
ch2 defb 0
	endstruct

; Nested structures:; metast1 is created with 2 sub-structures st1 called pr1 et pr2
struct metast1
	struct st1 pr1
	struct st1 pr2
endstruct

struct metast1 mymeta

	LD HL,mymeta.pr2.ch1
	LD A,(HL)