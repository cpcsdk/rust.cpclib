machin equ 5
ifused machin : assert 0==1 : endif

truc=60
ifused truc : assert 0==1 : endif

labeltrois
ifused labeltrois : assert 0==1 : endif

; le IFUSED ne déclenche pas l'usage
ifused machin : assert 0==1 : endif
ifused truc : assert 0==1 : endif
ifused labeltrois : assert 0==1 : endif



noexport labeltrois
ifused labeltrois : assert 0==1 : endif
;enoexport labeltrois
;ifused labeltrois : assert 0==1 : endif


ld a,machin
ld hl,truc
ld de,labeltrois



; variable, alias, label utilisés
ifnused machin : assert 0==1 : endif
ifnused truc : assert 0==1 : endif
ifnused labeltrois : assert 0==1 : endif