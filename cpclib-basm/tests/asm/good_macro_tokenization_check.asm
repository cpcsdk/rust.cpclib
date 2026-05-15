 
 repeat 3, channelNumber
    ld (PLY_AKG_PSGReg{{channelNumber} + 7}),a ;Reaches register/label 8/9/10
 endr

PLY_AKG_PSGReg8
PLY_AKG_PSGReg9
PLY_AKG_PSGReg10
