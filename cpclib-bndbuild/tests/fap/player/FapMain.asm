;
;       Fucking Fast AY Player - 2024
;         by Hicks/Vanity and Gozeur
;

        org	#0000
        
        include	"FapMacro.asm"

FapPlay:
        include	"FapPlay.asm"
        print	"FAP Player size: ", $-FapPlay
        save	"Build/fap-play.bin", FapPlay, $-FapPlay

FapInit:
        include	"FapInit.asm"
        print	"FAP Init code size: ", $-FapInit
        save	"Build/fap-init.bin", FapInit, $-FapInit
