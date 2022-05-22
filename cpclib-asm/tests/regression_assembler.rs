use std::ops::Deref;

use cpclib_asm::assemble;
use cpclib_asm::preamble::{
    parse_crunched_section, parse_z80_line_complete, parse_z80_line_label_aware_directive,
    ParserContext, Z80Span
};

lazy_static::lazy_static! {
    static ref CTX: ParserContext = Default::default();
}

fn ctx() -> &'static ParserContext {
    &CTX
}

fn ctx_and_span(code: &'static str) -> (Box<ParserContext>, Z80Span) {
    let mut ctx = Box::new(ParserContext::default());
    ctx.source = Some(code);
    ctx.context_name = Some("TEST".into());
    let span = Z80Span::new_extra(code, ctx.deref());
    (ctx, span)
}

#[test]
pub fn assemble_vsync_test() {
    let code = "
	org 0x4000
	ld b, 0xf5
loop
	in a, (c)
	rra
	jr nc, loop
finish
	assert finish == 0x4000 + (3+1+1+2)
	jr $
	";

    let binary = assemble(code, ctx()).unwrap();

    assert_eq!(
        &binary,
        &[0x06, 0xF5, 0xED, 0x78, 0x1F, 0x30, 0xFB, 0x18, 0xFE]
    );
}

#[test]
pub fn macro_local_labels() {
    let code = "
	MACRO CRC32XOR x1,x2,x3,x4
	rr b
	jr nc,@nextBit
	  ld a,e
	  xor {x1}
	  ld e,a
	  ld a,d
	  xor {x2}
	  ld d,a
	  ld a,l
	  xor {x3}
	  ld l,a
	  ld a,h
	  xor {x4}
	  ld h,a
@nextBit
  MEND

	       CRC32XOR &2C,&61,&0E,&EE
		   CRC32XOR &19,&C4,&6D,&07
	";

    // just check that it assemble
    let binary = assemble(code, ctx()).unwrap();
    assert!(binary.len() != 0);
}

#[test]
#[ignore = "currently failed. Need to implemente in a different way struct constructiion with default values"]
pub fn test_inner_struct1() {
    let code = "
	struct point
xx    db 4
yy    db 5
zz    db 6
	  endstruct

; each point uses the default values if nothing is provided
	struct triangle
p1 point 
p2 point 
p3 point 
	endstruct


; triangle with default values (4,5,6) for each point
my_triangle1: triangle
	";

    // just check that it assemble
    let binary = assemble(code, ctx()).unwrap();
    assert_eq!(binary.len(), 3 * 3);
    assert_eq!(&binary, &[4, 5, 6, 4, 5, 6, 4, 5, 6,])
}

#[test]
pub fn test_inner_struct2() {
    let code = "
	struct point
xx    db 4
yy    db 5
zz    db 6
	  endstruct

	struct triangle
p1 point 1, 2 , 3
p2 point ,,8
p3 point 9
	endstruct


my_triangle1: triangle [2, 3, 4], [10, 20, 30], [100, 200, 255]
	";

    // just check that it assemble
    let binary = assemble(code, ctx()).unwrap();
    assert_eq!(binary.len(), 3 * 3);
    assert_eq!(&binary, &[2, 3, 4, 10, 20, 30, 100, 200, 255])
}

#[test]
pub fn test_inner_struct3() {
    let code = "
	struct point
xx    db 4
yy    db 5
zz    db 6
	  endstruct

	struct triangle
p1 point 1, 2 , 3
p2 point ,,8
p3 point 9
	endstruct


my_triangle1: triangle [11, 12, 13],, [1, 2, 3]
	";

    // just check that it assemble
    let binary = assemble(code, ctx()).unwrap();
    assert_eq!(binary.len(), 3 * 3);
    assert_eq!(&binary, &[11, 12, 13, 4, 5, 8, 1, 2, 3,])
}

#[test]
#[ignore = "Need to better handle default case"]
pub fn test_inner_struct_deeper() {
    let code = "
	org 0x0000
	struct point
xx    db 4
yy    db 5
zz    db 6
	  endstruct

	struct triangle
p1 point 1, 2 , 3
p2 point ,,8
p3 point 9
	endstruct


	struct shape
tr1 triangle
tr2 triangle
	endstruct
	
	
	
my_shape: shape	
";

    // just check that it assemble
    let binary = assemble(code, ctx()).unwrap();
    assert_eq!(
        &binary,
        &[1, 2, 3, 4, 5, 8, 9, 5, 6, 1, 2, 3, 4, 5, 8, 9, 5, 6,]
    )
}

#[test]
#[ignore = "Need to better handle default case"]
pub fn test_inner_struct_deeper2() {
    let code = "
	struct point
xx    db 4
yy    db 5
zz    db 6
	  endstruct

	struct triangle
p1 point 1, 2 , 3
p2 point ,,8
p3 point 9
	endstruct


	struct shape
tr1 triangle
tr2 triangle
	endstruct
	
	
	
my_shape: shape	, [ [1,2,3], [1,2,3], [1,2,3] ]
";

    // just check that it assemble
    let binary = assemble(code, ctx()).unwrap();
    assert_eq!(binary.len(), 3 * 3 * 2);
    assert_eq!(
        &binary,
        &[1, 2, 3, 4, 5, 8, 9, 5, 6, 1, 2, 3, 1, 2, 3, 1, 2, 3,]
    )
}

#[test]
fn regression_crunched_section_sokoban() {
    let code = "


ENTITY_EMPTY = 1

ENTITY_FLOOR = 0
ENTITY_DESTINATION = 2
DEST_BIT = 1
ENTITY_BLOC = 4 ; ALWAYS in addition of floor or destination
BLOC_BIT = 2

ENTITY_VOID = 3

ENTITY_WALL = 8
WALL_BIT = 3
ENTITY_PLAYER = 16



	macro MAP_CHECK_BLOC bloc
		assert {bloc} == ENTITY_EMPTY || {bloc} == ENTITY_BLOC || {bloc} == ENTITY_DESTINATION || {bloc} == ENTITY_FLOOR || {bloc} == ENTITY_WALL || {bloc} == ENTITY_PLAYER || {bloc} == BD || {bloc} == ENTITY_VOID
	mend
	
	;;
	; Safely produce the data for a line of the map
	macro MAP_LINE9 a, b, c, d, e, f, g, h, i
		MAP_CHECK_BLOC {a}
		MAP_CHECK_BLOC {b}
		MAP_CHECK_BLOC {c}
		MAP_CHECK_BLOC {d}
		MAP_CHECK_BLOC {e}
		MAP_CHECK_BLOC {f}
		MAP_CHECK_BLOC {g}
		MAP_CHECK_BLOC {h}
		MAP_CHECK_BLOC {i}

		db E_
		db E_

		db {a}
		db {b}
		db {c}
		db {d}
		db {e}
		db {f}
		db {g}
		db {h}
		db {i}


		db E_
	mend

	macro MAP_LINE12 a, b, c, d, e, f, g, h, i,j,k,l
		MAP_CHECK_BLOC {a}
		MAP_CHECK_BLOC {b}
		MAP_CHECK_BLOC {c}
		MAP_CHECK_BLOC {d}
		MAP_CHECK_BLOC {e}
		MAP_CHECK_BLOC {f}
		MAP_CHECK_BLOC {g}
		MAP_CHECK_BLOC {h}
		MAP_CHECK_BLOC {i}

		db {a}
		db {b}
		db {c}
		db {d}
		db {e}
		db {f}
		db {g}
		db {h}
		db {i}

		db {j}
		db {k}
		db {l}
	mend

W_ = ENTITY_WALL
F_ = ENTITY_FLOOR
B_ = ENTITY_BLOC
D_ = ENTITY_DESTINATION
E_ = ENTITY_EMPTY
P_ = ENTITY_PLAYER


V_ = ENTITY_VOID

BD = B_ + D_

	macro MAP_EMPTY_LINE
		MAP_LINE9 E_,E_,E_,E_,E_,E_,E_,E_,E_
	mend

    LZAPU
.player_y db 5
.player_x db 6
        MAP_LINE12 E_,E_,E_,E_,W_,W_,W_,W_,W_,E_,E_,E_
        MAP_LINE12 E_,E_,W_,W_,W_,F_,F_,F_,W_,E_,E_,E_
        MAP_LINE12 E_,E_,W_,F_,F_,BD,W_,F_,W_,W_,E_,E_
        MAP_LINE12 E_,E_,W_,F_,W_,F_,F_,BD,F_,W_,E_,E_
        MAP_LINE12 E_,E_,W_,F_,BD,F_,F_,W_,F_,W_,E_,E_
        MAP_LINE12 E_,E_,W_,W_,F_,W_,D_,F_,F_,W_,E_,E_
        MAP_LINE12 E_,E_,E_,W_,F_,F_,F_,B_,W_,W_,E_,E_
        MAP_LINE12 E_,E_,E_,W_,W_,W_,F_,F_,W_,E_,E_,E_
        MAP_LINE12 E_,E_,E_,E_,E_,W_,W_,W_,W_,E_,E_,E_
        LZCLOSE
";

    let bin = dbg!(assemble(code, ctx()));
    assert!(bin.is_ok());
}

#[test]
fn label_colon_equ() {
    let code = "PLY_AKY_OPCODE_OR_A: equ #b7";
    let (ctx, span) = ctx_and_span(code);

    assert!(dbg!(parse_z80_line_label_aware_directive(span.clone())).is_ok());

    assert!(dbg!(parse_z80_line_complete(span)).is_ok());
}

#[test]
fn lzclose() {
    let code = "LZX0
	INNER_START
			defs 100
	INNER_STOP
		LZCLOSE";
    let (ctx, span) = ctx_and_span(code);

    assert!(dbg!(parse_crunched_section(span.clone())).is_ok());

    assert!(dbg!(parse_z80_line_complete(span)).is_ok());
}
