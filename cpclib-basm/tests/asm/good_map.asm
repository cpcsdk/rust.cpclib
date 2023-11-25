	; extract stolen here https://github.com/GuillianSeed/MetalGear/blob/master/Variables.asm#L10
		    map	#c000
		    
GameStatus:	    # 1
GameSubstatus:	    # 1
ControlConfig:	    # 1
							    ; Bit6: 1=Enable music/Player control
TickCounter:	    # 1
WaitCounter:	    # 1
TickInProgress:	    # 1
ControlsTrigger:    # 1
							    ; 5	= Fire2	/ M,  4	= Fire / Space,	3 = Right, 2 = Left, 1 = Down, 0 = Up
ControlsHold:	    # 1
							    ; 5	= Fire2	/ M,  4	= Fire / Space,	3 = Right, 2 = Left, 1 = Down, 0 = Up
Pause_1_F5_2:	    # 1
TutorialStatus:	    # 1
DemolHoldTime:	    # 1
UnusedVar1:	    # 1
DemoPlayId:	    # 1
DemoDataPointer:    # 2
							    ; Pointer to presaved demo controls
SprShuffleOffset:   # 1


	assert GameStatus = 0xc000
	assert GameSubstatus = (0xc000 + 1)
	assert ControlConfig = (0xc000 + 1 + 1)
	assert TickCounter = (0xc000 + 1 + 1+1)
	assert DemolHoldTime = (0xc000 + 10)
	assert SprShuffleOffset = (0xc000 + 15)