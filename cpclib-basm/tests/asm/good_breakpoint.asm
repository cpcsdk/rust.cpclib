
	org 0x1000

	breakpoint
	breakpoint 0x2000

label
	defb "Breakpoints parsing test"

	assert label == &1000
	breakpoint label+5


	breakpoint type=mem
	breakpoint type=mem, access=write
	breakpoint type=mem, access=write, runmode=stop, mask=&ff00
	breakpoint type=mem, access=write, runmode=stop, mask=&ff00, size=10
	breakpoint type=mem, access=write, runmode=stop, mask=&ff00, size=10, value=5
	breakpoint type=mem, access=write, runmode=stop, mask=&ff00, size=10, value=5, valmask=5
	breakpoint type=mem, access=write, runmode=stop, mask=&ff00, size=10, value=5, valmask=5, condition="HL=5"
	breakpoint type=mem, access=write, runmode=stop, mask=&ff00, size=10, value=5, valmask=5, condition="HL=5", name="hello"
	breakpoint type=mem, access=write, runmode=stop, mask=&ff00, size=10, value=5, valmask=5, condition="HL=5", name="hello", step=30