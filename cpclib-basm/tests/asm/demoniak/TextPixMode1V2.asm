        ORG     #a400
TabPt   EQU     #200

        DI
        LD      IX,TabPt                ; Adresse des points à afficher
        LD      IY,Vide                 ; Adresse d'un buffer "vide"

debutprog:
        LD      B,#F5
Sync:
        IN      A,(C)                   ; Attendre la VBL
        RRA
        JR      NC,SynC
        LD      BC,#7F10
        LD      A,#54
        OUT     (C),C
        OUT     (C),A                   ; Border noir
debut:
        LD      D,#A5                   ; Adresse du masque des points = #A500
        LD      BC,#C0                  ; C = adresse début mémoire vidéo, B = 0

;
; Efface l'ancienne image
;
Clear:
        LD      A,(IY+1)                ; Adresse poids fort mémoire vidéo à effacer
        LD      E,A
        LD      L,(IY+0)                ; Adresse poids faible mémoire vidéo à éffacer
        OR      L                       ; Y-a-t-il encore des points à effacer ?
        JR      Z,FinClear              ; Sinon, fini !
        LD      A,E
        OR      C                       ; Pour adresse mémoire vidéo débutant en #C000
        LD      H,A
        LD      (HL),B                  ; Efface le point
        INC     IY
        INC     IY
        JR      Clear                   ; Passer au point suivant
FinClear:
        PUSH    IX                      ; On a fini, d'effacer l'ancienne image,
        POP     IY                      ; On fait Ancienne image = nouvelle image


Trace:
        LD      A,(IX+1)                ; Adresse poids fort mémoire vidéo à afficher
        LD      E,A                     ; bits 7 et 6 de E = numéro octet
        LD      L,(IX+0)                ; Adresse poids faible mémoire vidéo à afficher
        OR      L
        JR      Z,Fin                   ; Adresse = 0 -> trame finie
        LD      A,E
        OR      C                       ; C = #C0 -> mettre adresse mémoire vidéo en #C000
        LD      H,A                     ; HL = adresse mémoire vidéo
        LD      A,(DE)                  ; récupère octet (#A500 à #A5FF)
        XOR      (HL)                   ; on aurai pu faire aussi un OR
        LD      (HL),A                  ; ou bien directement un LD (HL),A
        INC     IX
        INC     IX                      ; passer au point suivant
        JR      Trace
fin:
        INC     IX
        INC     IX
        LD      A,(IX+0)                ; Fin animation ?
        LD      B,(IX+1)
        OR      B
        JR      NZ,FinSuiv

        LD      IX,TabPt                ; Réinitialise pointeur début animation

FinSuiv:
        LD      BC,#7F10
        LD      A,#4B
        OUT     (C),C                   ; Border en blanc (pour mesure VBL)
        OUT     (C),A
        CALL    TSPACE                  ; Test touche Espace
        RLA
        RET     C                       ; Retour si espace appuyé
        JR      DebutProg               ; Sinon, on boucle 


;
; Test la touche espace avec le PPI... Du classique...
;
TSPACE:
        LD      A,#45
TCLAV:
        LD      (TCLAV1+1),A:; A=N. LIGNE 
        XOR     A
        LD      BC,#F40E
        OUT     (C),C
        LD      BC,#F6C0
        OUT     (C),C
        OUT     (C),A
        LD      BC,#F792
        OUT     (C),C
TCLAV1:
        LD      BC,#F645
        OUT     (C),C
        LD      B,#F4
        IN      A,(C)
        CPL
        LD      BC,#F782
        OUT     (C),C
        RET

;
; Adresse vide pour la première image: rien à effacer
;
Vide:
        DB      0,0


;
; Table des valeurs des octets pour chaque points
;
        ORG     #A500

        DS      64,#88          ; Points en x = 0
        DS      64,#44          ; Points en x = 1
        DS      64,#22          ; Points en x = 2
        DS      64,#11          ; Points en x = 3


