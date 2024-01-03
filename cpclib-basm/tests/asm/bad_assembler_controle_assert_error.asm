/**
 * This exemple illustrate how to :
 * - force display of some information at parsing pass (mainly usefull for debuging purposes, o,ly strings can be provided)
 * - force display of some information at each assembling pass
 * - limit the number of pass at some places of the code (parser is fucking slow, it is interesting to do that in location where LOTS of macros are interpreted wheras they produce the very same thing at each pass)
 *
 */


	org 0x4000


	ASMCONTROL PRINT_PARSE, "I AM PARSING THE BEGINNING OF SOURCE FILE"


	PRINT "MACRO1 is supposed to be parsed 2 times and assembled 2 times"
	PRINT "MACRO2 is supposed to be parsed 1 time and assembled 1 time"
	PRINT "MACRO3 is supposed to be parsed 1 time and assembled 1 time"
	PRINT "MACRO4 is supposed to be parsed 2 times and assembled 2 times"

TEST_MACRO macro, nb
	ASMCONTROL PRINT_ANY_PASS, "$=", {hex}$
	print "$=", {hex}$
	ASMCONTROL PRINT_PARSE, "I AM PARSING THE CODE OF THE MACRO{nb}"
	ASMCONTROL PRINT_ANY_PASS, "I AM DOING A PASS ON THE MACRO", {nb}
	db {nb}
	endm
	
	ld hl, label_whose_value_is_provided_during_second_pass ; 3 bytes

	TEST_MACRO 1 ; 1 byte

	; Here, we ensure only ONE pass is used to assemble this part WHEREAS the whole source is assembled in TWO passes.
	; There are some limitations and probably bugs too. For example to use org directive is probably not hnadled properly
	ASMCONTROLENV SET_MAX_NB_OF_PASSES=1
		org $+2
		assert $ == 0x4000 + 3 + 1 + 2 + 100 ; here it is wrong we expect a failure
		TEST_MACRO 2 ; 1 byte
		TEST_MACRO 3 ; 1 byte
	ENDASMCONTROLENV

	TEST_MACRO 4 ; 1 byte


label_whose_value_is_provided_during_second_pass

	print {hex}$
	print memory(0x4000+3) 
	print memory(0x4001+3) 
	print memory(0x4002+3) 
	print memory(0x4003+3) 
	print memory(0x4004+3) 
	print memory(0x4005+3) 

	assert memory(0x4000+3) == 1
	assert memory(0x4001+3+0) == 0
	assert memory(0x4001+3+1) == 0
	assert memory(0x4001+3+2) == 2
	assert memory(0x4002+3+2) == 3
	assert memory(0x4003+3+2) == 4
	assert  $ == 0x4000 + 3 + 4 + 2

	ASMCONTROL PRINT_PARSE, "I AM PARSING THE END OF SOURCE FILE"
