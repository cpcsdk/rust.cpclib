	
	

	
	write direct -1,-1,&c0
    org &4000
START_DATA_BK0
	defb 'C0'
END_DATA_BK0
	save "BANK_C0.TXT", START_DATA_BK0, (END_DATA_BK0-START_DATA_BK0)

	
	write direct -1,-1,&c4
    org &4000
START_DATA_BK4
	defb 'C4'
END_DATA_BK4
	save "BANK_C4.TXT", START_DATA_BK4, (END_DATA_BK4-START_DATA_BK4)

	
	write direct -1,-1,&c5
    org &4000
START_DATA_BK5
	defb 'C5'
END_DATA_BK5
	save "BANK_C5.TXT", START_DATA_BK5, (END_DATA_BK5-START_DATA_BK5)



	
	write direct -1,-1,&c6
    org &4000
START_DATA_BK6
	defb 'C6'
END_DATA_BK6
	save "BANK_C6.TXT", START_DATA_BK6, (END_DATA_BK6-START_DATA_BK6)
	
	
	write direct -1,-1,&c7
    org &4000
START_DATA_BK7
	defb 'C7'
END_DATA_BK7
	save "BANK_C7.TXT", START_DATA_BK7, (END_DATA_BK7-START_DATA_BK7)