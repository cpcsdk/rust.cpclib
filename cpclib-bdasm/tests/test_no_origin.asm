label_0132 EQU 0x132
label_00ab EQU 0xab
label_01fd EQU 0x1fd
label_0241 EQU 0x241
label_0165 EQU 0x165
label_0222 EQU 0x222
label_0251 EQU 0x251
label_0258 EQU 0x258
label_2133 EQU 0x2133
label_024b EQU 0x24b
label_0253 EQU 0x253
label_0094 EQU 0x94
label_00cc EQU 0xcc
label_3535 EQU 0x3535
label_00b9 EQU 0xb9
label_0085 EQU 0x85
label_008b EQU 0x8b
label_7420 EQU 0x7420
label_200a EQU 0x200a
label_0101 EQU 0x101
label_027a EQU 0x27a
label_0105 EQU 0x105
label_005d EQU 0x5d
label_6420 EQU 0x6420
label_0287 EQU 0x287
label_00ff EQU 0xff
label_007b EQU 0x7b
label_0220 EQU 0x220
label_0271 EQU 0x271
label_023f EQU 0x23f
label_0260 EQU 0x260
label_026a EQU 0x26a
label_0243 EQU 0x243
	ORG 0x0
	DEC SP
	JR NZ, label_005f
	JR C, label_0037
	JR NZ, label_0051
	LD H, L
	LD L, H
	LD L, H
	LD L, A
	JR NZ, label_0066
	LD L, A
	LD (HL), D
	LD L, H
	LD H, H
	JR NZ, label_007b
	LD L, A
	LD (HL), D
	JR NZ, label_005a
	LD L, L
	LD (HL), E
	LD (HL), H
	LD (HL), D
	LD H, C
	LD H, H
	JR NZ, label_0064
	LD D, B
	LD B, E
	LD A, (BC)
	DEC SP
	JR NZ, label_007b
	LD L, B
	LD L, C
	LD (HL), E
	JR NZ, label_009c
	LD (HL), D
	LD L, A
	LD H, A
	LD (HL), D
	LD H, C
	LD L, L
	JR NZ, label_00a4
	LD (HL), D
	LD L, C
	LD L, (HL)
	LD (HL), H
	LD (HL), E
label_0037
	JR NZ, label_005d
	LD C, B
	LD H, L
	LD L, H
	LD L, H
	LD L, A
	JR NZ, label_0099
	LD L, A
	LD (HL), D
	LD L, H
	LD H, H
	JR NZ, label_007b
	LD (label_2133), A
	LD (label_7420), HL
	LD L, A
	JR NZ, label_00c5
	LD L, B
	LD H, L
label_0051
	JR NZ, label_00c8
	LD H, E
	LD (HL), D
	LD H, L
	LD H, L
	LD L, (HL)
	LD A, (BC)
	LD A, (BC)
label_005a
	JR NZ, label_007e
	JR NZ, label_0080
	LD D, B
label_005f
	LD (HL), D
	LD L, C
	LD L, (HL)
	LD (HL), H
	LD B, E
label_0064
	LD L, B
	LD H, C
label_0066
	LD (HL), D
	JR NZ, label_008b
	JR NZ, label_008d
	JR NZ, label_00d4
	LD (HL), C
	LD (HL), L
	JR NZ, label_0099
	LD B, D
	LD B, D
	DEC (HL)
	LD B, C
	JR NZ, label_0099
	JR NZ, label_0085
	LD A, (BC)
	JR NZ, label_009e
	JR NZ, label_00a0
label_007e
	LD L, A
	LD (HL), D
label_0080
	LD H, A
	JR NZ, label_00ab
	LD SP, 0x3032
	JR NC, label_0094
	JR NZ, label_00ac
	JR NZ, label_00ae
	LD (HL), D
label_008d
	LD (HL), L
	LD L, (HL)
	JR NZ, label_00b7
	LD A, (BC)
	LD A, (BC)
	JR NZ, label_00b7
	JR NZ, label_00b9
	LD L, H
	LD H, H
label_0099
	JR NZ, label_0105
	LD L, H
label_009c
	INC L
	LD C, L
label_009e
	LD H, L
	LD (HL), E
label_00a0
	LD (HL), E
	LD H, C
	LD H, A
	LD H, L
label_00a4
	JR NZ, label_00c8
	JR NZ, label_00ca
	JR NZ, label_00cc
	JR NZ, label_00ce
label_00ac
	JR NZ, label_00d0
label_00ae
	JR NZ, label_00d2
	DEC SP
	LD B, C
	LD H, H
	LD H, H
	LD (HL), D
	LD H, L
	LD (HL), E
label_00b7
	LD (HL), E
	JR NZ, label_012b
	LD H, (HL)
	JR NZ, label_0132
	LD (HL), H
	LD (HL), D
	LD L, C
	LD L, (HL)
	LD H, A
	LD A, (BC)
	JR NZ, label_00e7
label_00c5
	JR NZ, label_00e9
	LD B, E
label_00c8
	LD H, C
	LD L, H
label_00ca
	LD L, H
	JR NZ, label_011f
	LD (HL), D
label_00ce
	LD L, C
	LD L, (HL)
label_00d0
	LD (HL), H
	LD D, E
label_00d2
	LD (HL), H
	LD (HL), D
label_00d4
	LD L, C
	LD L, (HL)
	LD H, A
	JR NZ, label_00fb
	JR NZ, label_00fd
	JR NZ, label_00ff
	JR NZ, label_0101
	DEC SP
	LD D, E
	LD L, B
	LD L, A
	LD (HL), A
	JR NZ, label_013b
	LD (HL), H
label_00e7
	LD (HL), D
	LD L, C
label_00e9
	LD L, (HL)
	LD H, A
	JR NZ, label_0163
	LD L, A
	JR NZ, label_0165
	LD H, E
	LD (HL), D
	LD H, L
	LD H, L
	LD L, (HL)
	LD A, (BC)
	LD A, (BC)
	JR NZ, label_011b
	JR NZ, label_011d
label_00fb
	LD (HL), D
	LD H, L
label_00fd
	LD (HL), H
	JR NZ, label_0122
	JR NZ, label_0124
	JR NZ, label_0126
	JR NZ, label_0128
	JR NZ, label_012a
	JR NZ, label_012c
	JR NZ, label_012e
	JR NZ, label_0130
	DEC SP
	LD B, (HL)
	LD L, C
	LD L, (HL)
	LD L, C
	LD (HL), E
	LD L, B
	LD H, L
	LD H, H
	JR NZ, label_0163
	LD H, L
	LD L, H
label_011b
	LD L, H
	LD L, A
label_011d
	JR NZ, label_0178
label_011f
	LD L, A
	LD (HL), D
	LD L, H
label_0122
	LD H, H
	LD A, (BC)
label_0124
	LD A, (BC)
	LD D, B
label_0126
	LD (HL), D
	LD L, C
label_0128
	LD L, (HL)
	LD (HL), H
label_012a
	LD D, E
label_012b
	LD (HL), H
label_012c
	LD (HL), D
	LD L, C
label_012e
	LD L, (HL)
	LD H, A
label_0130
	LD A, (label_200a)
	JR NZ, label_0157
	JR NZ, label_01a5
	LD H, H
	JR NZ, label_019d
	INC L
label_013b
	JR Z, label_01a7
	LD L, H
	ADD HL, HL
	JR NZ, label_0163
	JR NZ, label_0165
	DEC SP
	LD D, B
	LD (HL), D
	LD L, C
	LD L, (HL)
	LD (HL), H
	JR NZ, label_01ae
	JR NZ, label_0176
	LD (label_3535), A
	DAA
	JR NZ, label_01c9
	LD H, L
	LD (HL), D
	LD L, L
	LD L, C
label_0157
	LD L, (HL)
	LD H, C
	LD (HL), H
	LD H, L
	LD H, H
	JR NZ, label_01d3
	LD (HL), H
	LD (HL), D
	LD L, C
	LD L, (HL)
	LD H, A
label_0163
	LD A, (BC)
	JR NZ, label_0188
	JR NZ, label_018a
	LD H, E
	LD (HL), B
	JR NZ, label_01a0
	DEC (HL)
	DEC (HL)
	LD A, (BC)
	JR NZ, label_0193
	JR NZ, label_0195
	LD (HL), D
	LD H, L
	LD (HL), H
label_0176
	JR NZ, label_01f4
label_0178
	LD A, (BC)
	JR NZ, label_019d
	JR NZ, label_019f
	LD L, C
	LD L, (HL)
	LD H, E
	JR NZ, label_01ec
	LD L, H
	LD A, (BC)
	JR NZ, label_01a8
	JR NZ, label_01aa
label_0188
	LD H, E
	LD H, C
label_018a
	LD L, H
	LD L, H
	JR NZ, label_01e0
	LD (HL), D
	LD L, C
	LD L, (HL)
	LD (HL), H
	LD B, E
label_0193
	LD L, B
	LD H, C
label_0195
	LD (HL), D
	LD A, (BC)
	JR NZ, label_01bb
	JR NZ, label_01bd
	LD L, D
	LD (HL), D
label_019d
	JR NZ, label_01f1
label_019f
	LD (HL), D
label_01a0
	LD L, C
	LD L, (HL)
	LD (HL), H
	LD D, E
	LD (HL), H
label_01a5
	LD (HL), D
	LD L, C
label_01a7
	LD L, (HL)
label_01a8
	LD H, A
	LD A, (BC)
label_01aa
	LD A, (BC)
	LD C, L
	LD H, L
	LD (HL), E
label_01ae
	LD (HL), E
	LD H, C
	LD H, A
	LD H, L
	LD A, (label_6420)
	LD H, D
	JR NZ, label_01e1
	LD C, B
	LD H, L
	LD L, H
label_01bb
	LD L, H
	LD L, A
label_01bd
	JR NZ, label_0218
	LD L, A
	LD (HL), D
	LD L, H
	LD H, H
	JR NZ, label_01fa
	LD (label_2133), A
	DAA
label_01c9
	INC L
	LD (label_3535), A
	LD A, (BC)
	LD A, (BC)
	LD C, (HL)
	LD H, L
	LD (HL), A
	LD C, H
label_01d3
	LD L, C
	LD L, (HL)
	LD H, L
	LD A, (label_200a)
	JR NZ, label_01fd
	JR NZ, label_024b
	LD H, H
	JR NZ, label_0243
label_01e0
	INC L
label_01e1
	LD SP, 0x2033
	JR NZ, label_0208
	JR NZ, label_020a
	JR NZ, label_020c
	JR NZ, label_0229
label_01ec
	LD B, E
	LD H, C
	LD (HL), D
	LD (HL), D
	LD L, C
label_01f1
	LD H, C
	LD H, A
	LD H, L
label_01f4
	JR NZ, label_026a
	LD H, L
	LD (HL), H
	LD (HL), L
	LD (HL), D
label_01fa
	LD L, (HL)
	LD A, (BC)
	JR NZ, label_0220
	JR NZ, label_0222
	LD H, E
	LD H, C
	LD L, H
	LD L, H
	JR NZ, label_0258
	LD (HL), D
	LD L, C
label_0208
	LD L, (HL)
	LD (HL), H
label_020a
	LD B, E
	LD L, B
label_020c
	LD H, C
	LD (HL), D
	LD A, (BC)
	JR NZ, label_0233
	JR NZ, label_0235
	LD L, H
	LD H, H
	JR NZ, label_027a
	INC L
label_0218
	LD SP, 0x2030
	JR NZ, label_023f
	JR NZ, label_0241
	JR NZ, label_0243
	JR NZ, label_0260
	LD C, H
	LD L, C
	LD L, (HL)
	LD H, L
	JR NZ, label_0271
label_0229
	LD H, L
	LD H, L
	LD H, H
	LD A, (BC)
	JR NZ, label_0251
	JR NZ, label_0253
	LD L, D
	LD (HL), B
label_0233
	JR NZ, label_0287
label_0235
	LD (HL), D
	LD L, C
	LD L, (HL)
	LD (HL), H
	LD B, E
	LD L, B
	LD H, C
	LD (HL), D
	LD A, (BC)
