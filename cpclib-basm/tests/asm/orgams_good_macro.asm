
	org #20

  MACRO ALIGN n
    SKIP -$ MOD n
  ENDM


  ALIGN 5
  ALIGN(5)


  MACRO NOARG
  ENDM
  
  NOARG()
  NOARG ()