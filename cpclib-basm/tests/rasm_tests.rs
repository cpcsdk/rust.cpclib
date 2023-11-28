/// Contains some tests stolen to rasm source code
/// Note that the autotest only check the parsing and not the execution in opposite to rasm.
/// I have not imported thos that rely on executation or those using a differnt syntax
/// Test that fail should be seen as modifications to improve to the parser
use std::process::Command;

#[derive(Default)]
struct VerifyOutput {
    size: Option<usize>,
    chk: Option<usize>,
    opcodes: Vec<(usize, u8)>
}

macro_rules! import_rasm_check {
    ($verif:ident,chk($chk:expr)) => {
        $verif.chk = Some($chk);
    };

    ($verif:ident,len($chk:expr)) => {
        $verif.size = Some($chk);
    };

    ($verif:ident,opcode0($chk:expr)) => {
        $verif.opcodes.push((0, $chk));
    };

    ($verif:ident,opcode1($chk:expr)) => {
        $verif.opcodes.push((1, $chk));
    };

    ($verif:ident,opcode2($chk:expr)) => {
        $verif.opcodes.push((2, $chk));
    };

    ($verif:ident,opcode3($chk:expr)) => {
        $verif.opcodes.push((3, $chk));
    };

    ($verif:ident,opcode4($chk:expr)) => {
        $verif.opcodes.push((4, $chk));
    };
}

macro_rules! import_rasm_success {

	($ (#define
		$name:ident
		$(:$test:ident($value:expr):)*
		$($code:expr)+
	   );+
		;
	)  => {$(
				#[ignore]
				#[test]
				fn $name() {
					let mut verif = VerifyOutput::default();
					$(
						//verif.chk = Some($chk);
						import_rasm_check!(verif, $test($value));
					)?
					assemble_success(concat!($($code),+), verif)
				}
		)+
	};




}

macro_rules! import_rasm_failure {
	($ (#define $name:ident $($code:expr)+);+ ;)  => {$(
				#[ignore]
				#[test]
				fn $name() {
					assemble_failure(concat!($($code),+), VerifyOutput::default())
				}
		)+
	}

	}

// Incompatible syntax
// #define AUTOTEST_BANKORG	"bank 0:nop:org #5:nop:bank 1:unevar=10:bank 0:assert $==6:ret:bank 1:assert $==0:bank 0:assert $==7";
//

// AUTOTEST_OPCODES is currently wrong. rasm  assembles impossible opcodes
import_rasm_success! {

    #define AUTOTEST_ORG
    :len(4):
    :opcode1(0x80):
    :opcode2(2):
    :opcode3(0x10):
    "ORG #8000,#1000:defw $:ORG $:defw $"
    ;
#define AUTOTEST_NOINCLUDE "truc equ 0:if truc:include'bite':endif:nop";

#define AUTOTEST_SWITCH		"mavar=4:switch mavar:case 1:nop:case 4:defb 4:case 3:defb 3:break:case 2:nop:case 4:defb 4:endswitch";

// I have removed instructions like bit 0,(ix+#12),d 
// seems not http://www.z80.info/z80undoc.htm
#define AUTOTEST_OPCODES "nop::ld bc,#1234::ld (bc),a::inc bc:inc b:dec b:ld b,#12:rlca:ex af,af':add hl,bc:ld a,(bc):dec bc:"
                         "inc c:dec c:ld c,#12:rrca::djnz $:ld de,#1234:ld (de),a:inc de:inc d:dec d:ld d,#12:rla:jr $:"
                         "add hl,de:ld a,(de):dec de:inc e:dec e:ld e,#12:rra::jr nz,$:ld hl,#1234:ld (#1234),hl:inc hl:inc h:"
                         "dec h:ld h,#12:daa:jr z,$:add hl,hl:ld hl,(#1234):dec hl:inc l:dec l:ld l,#12:cpl::jr nc,$:"
                         "ld sp,#1234:ld (#1234),a:inc sp:inc (hl):dec (hl):ld (hl),#12:scf:jr c,$:add hl,sp:ld a,(#1234):"
                         "dec sp:inc a:dec a:ld a,#12:ccf::ld b,b:ld b,c:ld b,d:ld b,e:ld b,h:ld b,l:ld b,(hl):ld b,a:ld c,b:"
                         "ld c,c:ld c,d:ld c,e:ld c,h:ld c,l:ld c,(hl):ld c,a::ld d,b:ld d,c:ld d,d:ld d,e:ld d,h:ld d,l:"
                         "ld d,(hl):ld d,a:ld e,b:ld e,c:ld e,d:ld e,e:ld e,h:ld e,l:ld e,(hl):ld e,a::ld h,b:ld h,c:ld h,d:"
                         "ld h,e:ld h,h:ld h,l:ld h,(hl):ld h,a:ld l,b:ld l,c:ld l,d:ld l,e:ld l,h:ld l,l:ld l,(hl):ld l,a::"
                         "ld (hl),b:ld (hl),c:ld (hl),d:ld (hl),e:ld (hl),h:ld (hl),l:halt:ld (hl),a:ld a,b:ld a,c:ld a,d:"
                         "ld a,e:ld a,h:ld a,l:ld a,(hl):ld a,a::add b:add c:add d:add e:add h:add l:add (hl):add a:adc b:"
                         "adc c:adc d:adc e:adc h:adc l:adc (hl):adc a::sub b:sub c:sub d:sub e:sub h:sub l:sub (hl):sub a:"
                         "sbc b:sbc c:sbc d:sbc e:sbc h:sbc l:sbc (hl):sbc a::and b:and c:and d:and e:and h:and l:and (hl):"
                         "and a:xor b:xor c:xor d:xor e:xor h:xor l:xor (hl):xor a::or b:or c:or d:or e:or h:or l:or (hl):"
                         "or a:cp b:cp c:cp d:cp e:cp h:cp l:cp (hl):cp a::ret nz:pop bc:jp nz,#1234:jp #1234:call nz,#1234:"
                         "push bc:add #12:rst 0:ret z:ret:jp z,#1234:nop:call z,#1234:call #1234:adc #12:rst 8::ret nc:pop de:"
                         "jp nc,#1234:out (#12),a:call nc,#1234:push de:sub #12:rst #10:ret c:exx:jp c,#1234:in a,(#12):"
                         "call c,#1234:nop:sbc #12:rst #18::ret po:pop hl:jp po,#1234:ex (sp),hl:call po,#1234:push hl:"
                         "and #12:rst #20:ret pe:jp (hl):jp pe,#1234:ex de,hl:call pe,#1234:nop:xor #12:rst #28::ret p:pop af:"
                         "jp p,#1234:di:call p,#1234:push af:or #12:rst #30:ret m:ld sp,hl:jp m,#1234:ei:call m,#1234:nop:"
                         "cp #12:rst #38:in b,(c):out (c),b:sbc hl,bc:ld (#1234),bc:neg:retn:im 0:ld i,a:in c,(c):out (c),c:"
                         "adc hl,bc:ld bc,(#1234):reti:ld r,a::in d,(c):out (c),d:sbc hl,de:ld (#1234),de:retn:im 1:ld a,i:"
                         "in e,(c):out (c),e:adc hl,de:ld de,(#1234):im 2:ld a,r::in h,(c):out (c),h:sbc hl,hl:rrd:in l,(c):"
                         "out (c),l:adc hl,hl:rld::in 0,(c):out (c),0:sbc hl,sp:ld (#1234),sp:in a,(c):out (c),a:adc hl,sp:"
                         "ld sp,(#1234)::ldi:cpi:ini:outi:ldd:cpd:ind:outd::ldir:cpir:inir:otir:lddr:cpdr:indr:otdr::rlc b:"
                         "rlc c:rlc d:rlc e:rlc h:rlc l:rlc (hl):rlc a:rrc b:rrc c:rrc d:rrc e:rrc h:rrc l:rrc (hl):rrc a::"
                         "rl b:rl c:rl d:rl e:rl h:rl l:rl (hl):rl a:rr b:rr c:rr d:rr e:rr h:rr l:rr (hl):rr a:sla b:sla c:"
                         "sla d:sla e:sla h:sla l:sla (hl):sla a:sra b:sra c:sra d:sra e:sra h:sra l:sra (hl):sra a::sll b:"
                         "sll c:sll d:sll e:sll h:sll l:sll (hl):sll a:srl b:srl c:srl d:srl e:srl h:srl l:srl (hl):srl a::"
                         "bit 0,b:bit 0,c:bit 0,d:bit 0,e:bit 0,h:bit 0,l:bit 0,(hl):bit 0,a::bit 1,b:bit 1,c:bit 1,d:bit 1,e:"
                         "bit 1,h:bit 1,l:bit 1,(hl):bit 1,a::bit 2,b:bit 2,c:bit 2,d:bit 2,e:bit 2,h:bit 2,l:bit 2,(hl):"
                         "bit 2,a::bit 3,b:bit 3,c:bit 3,d:bit 3,e:bit 3,h:bit 3,l:bit 3,(hl):bit 3,a::bit 4,b:bit 4,c:"
                         "bit 4,d:bit 4,e:bit 4,h:bit 4,l:bit 4,(hl):bit 4,a::bit 5,b:bit 5,c:bit 5,d:bit 5,e:bit 5,h:bit 5,l:"
                         "bit 5,(hl):bit 5,a::bit 6,b:bit 6,c:bit 6,d:bit 6,e:bit 6,h:bit 6,l:bit 6,(hl):bit 6,a::bit 7,b:"
                         "bit 7,c:bit 7,d:bit 7,e:bit 7,h:bit 7,l:bit 7,(hl):bit 7,a::res 0,b:res 0,c:res 0,d:res 0,e:res 0,h:"
                         "res 0,l:res 0,(hl):res 0,a::res 1,b:res 1,c:res 1,d:res 1,e:res 1,h:res 1,l:res 1,(hl):res 1,a::"
                         "res 2,b:res 2,c:res 2,d:res 2,e:res 2,h:res 2,l:res 2,(hl):res 2,a::res 3,b:res 3,c:res 3,d:res 3,e:"
                         "res 3,h:res 3,l:res 3,(hl):res 3,a::res 4,b:res 4,c:res 4,d:res 4,e:res 4,h:res 4,l:res 4,(hl):"
                         "res 4,a::res 5,b:res 5,c:res 5,d:res 5,e:res 5,h:res 5,l:res 5,(hl):res 5,a::res 6,b:res 6,c:"
                         "res 6,d:res 6,e:res 6,h:res 6,l:res 6,(hl):res 6,a::res 7,b:res 7,c:res 7,d:res 7,e:res 7,h:res 7,l:"
                         "res 7,(hl):res 7,a::set 0,b:set 0,c:set 0,d:set 0,e:set 0,h:set 0,l:set 0,(hl):set 0,a::set 1,b:"
                         "set 1,c:set 1,d:set 1,e:set 1,h:set 1,l:set 1,(hl):set 1,a::set 2,b:set 2,c:set 2,d:set 2,e:set 2,h:"
                         "set 2,l:set 2,(hl):set 2,a::set 3,b:set 3,c:set 3,d:set 3,e:set 3,h:set 3,l:set 3,(hl):set 3,a::"
                         "set 4,b:set 4,c:set 4,d:set 4,e:set 4,h:set 4,l:set 4,(hl):set 4,a::set 5,b:set 5,c:set 5,d:set 5,e:"
                         "set 5,h:set 5,l:set 5,(hl):set 5,a::set 6,b:set 6,c:set 6,d:set 6,e:set 6,h:set 6,l:set 6,(hl):"
                         "set 6,a::set 7,b:set 7,c:set 7,d:set 7,e:set 7,h:set 7,l:set 7,(hl):set 7,a::add ix,bc::add ix,de::"
                         "ld ix,#1234:ld (#1234),ix:inc ix:inc xh:dec xh:ld xh,#12:add ix,ix:ld ix,(#1234):dec ix:inc xl:"
                         "dec xl:ld xl,#12::inc (ix+#12):dec (ix+#12):ld (ix+#12),#34:add ix,sp::ld b,xh:ld b,xl:"
                         "ld b,(ix+#12):ld c,xh:ld c,xl:ld c,(ix+#12):::ld d,xh:ld d,xl:ld d,(ix+#12):ld e,xh:ld e,xl:"
                         "ld e,(ix+#12)::ld xh,b:ld xh,c:ld xh,d:ld xh,e:ld xh,xh:ld xh,xl:ld h,(ix+#12):ld xh,a:ld xl,b:"
                         "ld xl,c:ld xl,d:ld xl,e:ld xl,xh:ld xl,xl:ld l,(ix+#12):ld xl,a::ld (ix+#12),b:ld (ix+#12),c:"
                         "ld (ix+#12),d:ld (ix+#12),e:ld (ix+#12),h:ld (ix+#12),l:ld (ix+#12),a:ld a,xh:ld a,xl:"
                         "ld a,(ix+#12)::add xh:add xl:add (ix+#12):adc xh:adc xl:adc (ix+#12)::sub xh:sub xl:sub (ix+#12):"
                         "sbc xh:sbc xl:sbc (ix+#12)::and xh:and xl:and (ix+#12):xor xh:xor xl:xor (ix+#12)::or xh:or xl:"
                         "or (ix+#12):cp xh:cp xl:cp (ix+#12)::pop ix:ex (sp),ix:push ix:jp (ix)::ld sp,ix:::rlc (ix+#12),b:"
                         "rlc (ix+#12),c:rlc (ix+#12),d:rlc (ix+#12),e:rlc (ix+#12),h:rlc (ix+#12),l:rlc (ix+#12):"
                         "rlc (ix+#12),a:rrc (ix+#12),b:rrc (ix+#12),c:rrc (ix+#12),d:rrc (ix+#12),e:rrc (ix+#12),h:"
                         "rrc (ix+#12),l:rrc (ix+#12):rrc (ix+#12),a::rl (ix+#12),b:rl (ix+#12),c:rl (ix+#12),d:rl (ix+#12),e:"
                         "rl (ix+#12),h:rl (ix+#12),l:rl (ix+#12):rl (ix+#12),a:rr (ix+#12),b:rr (ix+#12),c:rr (ix+#12),d:"
                         "rr (ix+#12),e:rr (ix+#12),h:rr (ix+#12),l:rr (ix+#12):rr (ix+#12),a::sla (ix+#12),b:sla (ix+#12),c:"
                         "sla (ix+#12),d:sla (ix+#12),e:sla (ix+#12),h:sla (ix+#12),l:sla (ix+#12):sla (ix+#12),a:"
                         "sra (ix+#12),b:sra (ix+#12),c:sra (ix+#12),d:sra (ix+#12),e:sra (ix+#12),h:sra (ix+#12),l:"
                         "sra (ix+#12):sra (ix+#12),a::sll (ix+#12),b:sll (ix+#12),c:sll (ix+#12),d:sll (ix+#12),e:"
                         "sll (ix+#12),h:sll (ix+#12),l:sll (ix+#12):sll (ix+#12),a:srl (ix+#12),b:srl (ix+#12),c:"
                         "srl (ix+#12),d:srl (ix+#12),e:srl (ix+#12),h:srl (ix+#12),l:srl (ix+#12):srl (ix+#12),a::"
                         "bit 0,(ix+#12):bit 1,(ix+#12):bit 2,(ix+#12):bit 3,(ix+#12):bit 4,(ix+#12):bit 5,(ix+#12):"
                         "res 0,(ix+#12),c:res 0,(ix+#12),d:res 0,(ix+#12),e:res 0,(ix+#12),h:res 0,(ix+#12),l:res 0,(ix+#12):"
                         "res 0,(ix+#12),a::res 1,(ix+#12),b:res 1,(ix+#12),c:res 1,(ix+#12),d:res 1,(ix+#12),e:"
                         "res 1,(ix+#12),h:res 1,(ix+#12),l:res 1,(ix+#12):res 1,(ix+#12),a::res 2,(ix+#12),b:"
                         "res 2,(ix+#12),c:res 2,(ix+#12),d:res 2,(ix+#12),e:res 2,(ix+#12),h:res 2,(ix+#12),l:res 2,(ix+#12):"
                         "res 2,(ix+#12),a::res 3,(ix+#12),b:res 3,(ix+#12),c:res 3,(ix+#12),d:res 3,(ix+#12),e:"
                         "res 3,(ix+#12),h:res 3,(ix+#12),l:res 3,(ix+#12):res 3,(ix+#12),a::res 4,(ix+#12),b:"
                         "res 4,(ix+#12),c:res 4,(ix+#12),d:res 4,(ix+#12),e:res 4,(ix+#12),h:res 4,(ix+#12),l:"
                         "res 4,(ix+#12):res 4,(ix+#12),a::res 5,(ix+#12),b:res 5,(ix+#12),c:res 5,(ix+#12),d:"
                         "res 5,(ix+#12),e:res 5,(ix+#12),h:res 5,(ix+#12),l:res 5,(ix+#12):res 5,(ix+#12),a::"
                         "res 6,(ix+#12),b:res 6,(ix+#12),c:res 6,(ix+#12),d:res 6,(ix+#12),e:res 6,(ix+#12),h:"
                         "res 6,(ix+#12),l:res 6,(ix+#12):res 6,(ix+#12),a::res 7,(ix+#12),b:res 7,(ix+#12),c:"
                         "res 7,(ix+#12),d:res 7,(ix+#12),e:res 7,(ix+#12),h:res 7,(ix+#12),l:res 7,(ix+#12):"
                         "res 7,(ix+#12),a::set 0,(ix+#12),b:set 0,(ix+#12),c:set 0,(ix+#12),d:set 0,(ix+#12),e:"
                         "set 0,(ix+#12),h:set 0,(ix+#12),l:set 0,(ix+#12):set 0,(ix+#12),a::set 1,(ix+#12),b:"
                         "set 1,(ix+#12),c:set 1,(ix+#12),d:set 1,(ix+#12),e:set 1,(ix+#12),h:set 1,(ix+#12),l:"
                         "set 1,(ix+#12):set 1,(ix+#12),a::set 2,(ix+#12),b:set 2,(ix+#12),c:set 2,(ix+#12),d:"
                         "set 2,(ix+#12),e:set 2,(ix+#12),h:set 2,(ix+#12),l:set 2,(ix+#12):set 2,(ix+#12),a::"
                         "set 3,(ix+#12),b:set 3,(ix+#12),c:set 3,(ix+#12),d:set 3,(ix+#12),e:set 3,(ix+#12),h:"
                         "set 3,(ix+#12),l:set 3,(ix+#12):set 3,(ix+#12),a::set 4,(ix+#12),b:set 4,(ix+#12),c:"
                         "set 4,(ix+#12),d:set 4,(ix+#12),e:set 4,(ix+#12),h:set 4,(ix+#12),l:set 4,(ix+#12):"
                         "set 4,(ix+#12),a::set 5,(ix+#12),b:set 5,(ix+#12),c:set 5,(ix+#12),d:set 5,(ix+#12),e:"
                         "set 5,(ix+#12),h:set 5,(ix+#12),l:set 5,(ix+#12):set 5,(ix+#12),a::set 6,(ix+#12),b:"
                         "set 6,(ix+#12),c:set 6,(ix+#12),d:set 6,(ix+#12),e:set 6,(ix+#12),h:set 6,(ix+#12),l:"
                         "set 6,(ix+#12):set 6,(ix+#12),a::set 7,(ix+#12),b:set 7,(ix+#12),c:set 7,(ix+#12),d:"
                         "set 7,(ix+#12),e:set 7,(ix+#12),h:set 7,(ix+#12),l:set 7,(ix+#12):set 7,(ix+#12),a::add iy,bc::"
                         "add iy,de::ld iy,#1234:ld (#1234),iy:inc iy:inc yh:dec yh:ld yh,#12:add iy,iy:ld iy,(#1234):dec iy:"
                         "inc yl:dec yl:ld yl,#12::inc (iy+#12):dec (iy+#12):ld (iy+#12),#34:add iy,sp::ld b,yh:ld b,yl:"
                         "ld b,(iy+#12):ld c,yh:ld c,yl:ld c,(iy+#12):::ld d,yh:ld d,yl:ld d,(iy+#12):ld e,yh:ld e,yl:"
                         "ld e,(iy+#12)::ld yh,b:ld yh,c:ld yh,d:ld yh,e:ld yh,yh:ld yh,yl:ld h,(iy+#12):ld yh,a:ld yl,b:"
                         "ld yl,c:ld yl,d:ld yl,e:ld yl,yh:ld yl,yl:ld l,(iy+#12):ld yl,a::ld (iy+#12),b:ld (iy+#12),c:"
                         "ld (iy+#12),d:ld (iy+#12),e:ld (iy+#12),h:ld (iy+#12),l:ld (iy+#12),a:ld a,yh:ld a,yl:"
                         "ld a,(iy+#12)::add yh:add yl:add (iy+#12):adc yh:adc yl:adc (iy+#12)::sub yh:sub yl:"
                         "sub (iy+#12):sbc yh:sbc yl:sbc (iy+#12)::and yh:and yl:and (iy+#12):xor yh:xor yl:xor (iy+#12)::"
                         "or yh:or yl:or (iy+#12):cp yh:cp yl:cp (iy+#12)::pop iy:ex (sp),iy:push iy:jp (iy)::ld sp,iy::"
                         "rlc (iy+#12),b:rlc (iy+#12),c:rlc (iy+#12),d:rlc (iy+#12),e:rlc (iy+#12),h:rlc (iy+#12),l:"
                         "rlc (iy+#12):rlc (iy+#12),a:rrc (iy+#12),b:rrc (iy+#12),c:rrc (iy+#12),d:rrc (iy+#12),e:"
                         "rrc (iy+#12),h:rrc (iy+#12),l:rrc (iy+#12):rrc (iy+#12),a::rl (iy+#12),b:rl (iy+#12),c:"
                         "rl (iy+#12),d:rl (iy+#12),e:rl (iy+#12),h:rl (iy+#12),l:rl (iy+#12):rl (iy+#12),a:rr (iy+#12),b:"
                         "rr (iy+#12),c:rr (iy+#12),d:rr (iy+#12),e:rr (iy+#12),h:rr (iy+#12),l:rr (iy+#12):rr (iy+#12),a::"
                         "sla (iy+#12),b:sla (iy+#12),c:sla (iy+#12),d:sla (iy+#12),e:sla (iy+#12),h:sla (iy+#12),l:"
                         "sla (iy+#12):sla (iy+#12),a:sra (iy+#12),b:sra (iy+#12),c:sra (iy+#12),d:sra (iy+#12),e:"
                         "sra (iy+#12),h:sra (iy+#12),l:sra (iy+#12):sra (iy+#12),a::sll (iy+#12),b:sll (iy+#12),c:"
                         "sll (iy+#12),d:sll (iy+#12),e:sll (iy+#12),h:sll (iy+#12),l:sll (iy+#12):sll (iy+#12),a:"
                         "srl (iy+#12),b:srl (iy+#12),c:srl (iy+#12),d:srl (iy+#12),e:srl (iy+#12),h:srl (iy+#12),l:"
                         "srl (iy+#12):srl (iy+#12),a::bit 0,(iy+#12):bit 1,(iy+#12):bit 2,(iy+#12):bit 3,(iy+#12):"
                         "bit 4,(iy+#12):bit 5,(iy+#12):bit 6,(iy+#12):bit 7,(iy+#12)::res 0,(iy+#12),b:res 0,(iy+#12),c:"
                         "res 0,(iy+#12),d:res 0,(iy+#12),e:res 0,(iy+#12),h:res 0,(iy+#12),l:res 0,(iy+#12):"
                         "res 0,(iy+#12),a::res 1,(iy+#12),b:res 1,(iy+#12),c:res 1,(iy+#12),d:res 1,(iy+#12),e:"
                         "res 1,(iy+#12),h:res 1,(iy+#12),l:res 1,(iy+#12):res 1,(iy+#12),a::res 2,(iy+#12),b:"
                         "res 2,(iy+#12),c:res 2,(iy+#12),d:res 2,(iy+#12),e:res 2,(iy+#12),h:res 2,(iy+#12),l:"
                         "res 2,(iy+#12):res 2,(iy+#12),a::res 3,(iy+#12),b:res 3,(iy+#12),c:res 3,(iy+#12),d:"
                         "res 3,(iy+#12),e:res 3,(iy+#12),h:res 3,(iy+#12),l:res 3,(iy+#12):res 3,(iy+#12),a::"
                         "res 4,(iy+#12),b:res 4,(iy+#12),c:res 4,(iy+#12),d:res 4,(iy+#12),e:res 4,(iy+#12),h:"
                         "res 4,(iy+#12),l:res 4,(iy+#12):res 4,(iy+#12),a::res 5,(iy+#12),b:res 5,(iy+#12),c:"
                         "res 5,(iy+#12),d:res 5,(iy+#12),e:res 5,(iy+#12),h:res 5,(iy+#12),l:res 5,(iy+#12):"
                         "res 5,(iy+#12),a::res 6,(iy+#12),b:res 6,(iy+#12),c:res 6,(iy+#12),d:res 6,(iy+#12),e:"
                         "res 6,(iy+#12),h:res 6,(iy+#12),l:res 6,(iy+#12):res 6,(iy+#12),a::res 7,(iy+#12),b:"
                         "res 7,(iy+#12),c:res 7,(iy+#12),d:res 7,(iy+#12),e:res 7,(iy+#12),h:res 7,(iy+#12),l:"
                         "res 7,(iy+#12):res 7,(iy+#12),a::set 0,(iy+#12),b:set 0,(iy+#12),c:set 0,(iy+#12),d:"
                         "set 0,(iy+#12),e:set 0,(iy+#12),h:set 0,(iy+#12),l:set 0,(iy+#12):set 0,(iy+#12),a::"
                         "set 1,(iy+#12),b:set 1,(iy+#12),c:set 1,(iy+#12),d:set 1,(iy+#12),e:set 1,(iy+#12),h:"
                         "set 1,(iy+#12),l:set 1,(iy+#12):set 1,(iy+#12),a::set 2,(iy+#12),b:set 2,(iy+#12),c:"
                         "set 2,(iy+#12),d:set 2,(iy+#12),e:set 2,(iy+#12),h:set 2,(iy+#12),l:set 2,(iy+#12):"
                         "set 2,(iy+#12),a::set 3,(iy+#12),b:set 3,(iy+#12),c:set 3,(iy+#12),d:set 3,(iy+#12),e:"
                         "set 3,(iy+#12),h:set 3,(iy+#12),l:set 3,(iy+#12):set 3,(iy+#12),a::set 4,(iy+#12),b:"
                         "set 4,(iy+#12),c:set 4,(iy+#12),d:set 4,(iy+#12),e:set 4,(iy+#12),h:set 4,(iy+#12),l:"
                         "set 4,(iy+#12):set 4,(iy+#12),a::set 5,(iy+#12),b:set 5,(iy+#12),c:set 5,(iy+#12),d:"
                         "set 5,(iy+#12),e:set 5,(iy+#12),h:set 5,(iy+#12),l:set 5,(iy+#12):set 5,(iy+#12),a::"
                         "set 6,(iy+#12),b:set 6,(iy+#12),c:set 6,(iy+#12),d:set 6,(iy+#12),e:set 6,(iy+#12),h:"
                         "set 6,(iy+#12),l:set 6,(iy+#12):set 6,(iy+#12),a::set 7,(iy+#12),b:set 7,(iy+#12),c:"
                         "set 7,(iy+#12),d:set 7,(iy+#12),e:set 7,(iy+#12),h:set 7,(iy+#12),l:set 7,(iy+#12):"
                         "set 7,(iy+#12),a:";


#define AUTOTEST_ENHANCED_LD	"ld h,(ix+11): ld l,(ix+10): ld h,(iy+21): ld l,(iy+20): ld b,(ix+11): ld c,(ix+10):"
"ld b,(iy+21): ld c,(iy+20): ld d,(ix+11): ld e,(ix+10): ld d,(iy+21): ld e,(iy+20): ld hl,(ix+10): "
"ld hl,(iy+20):ld bc,(ix+10):ld bc,(iy+20): ld de,(ix+10):ld de,(iy+20)";

#define AUTOTEST_ENHANCED_PUSHPOP	"push bc,de,hl,ix,iy,af:pop hl,bc,de,iy,ix,af:nop 2:"
                "push bc:push de:push hl:push ix:push iy:push af:"
                "pop hl:pop bc:pop de:pop iy:pop ix:pop af:nop:nop";

#define AUTOTEST_ENHANCED_LD2 "ld (ix+0),hl: ld (ix+0),de: ld (ix+0),bc: ld (iy+0),hl: ld (iy+0),de: ld (iy+0),bc:"
"ld (ix+1),h: ld (ix+0),l: ld (ix+1),d: ld (ix+0),e: ld (ix+1),b: ld (ix+0),c: ld (iy+1),h: ld (iy+0),l: ld (iy+1),d: ld (iy+0),e: ld (iy+1),b: ld (iy+0),c";

#define AUTOTEST_ENHANCED_LD3 "ld bc,ix : ld bc,iy : ld de,ix : ld de,iy : ld ix,bc : ld ix,de : ld iy,bc : ld iy,de:"
    "ld b,hx : ld c,lx : ld b,hy : ld c,ly : ld d,hx : ld e,lx : ld d,hy : ld e,ly :"
    "ld hx,b : ld lx,c : ld hx,d : ld lx,e : ld hy,b : ld ly,c : ld hy,d : ld ly,e";

#define AUTOTEST_OPERATORASSIGN "a=5 : a*=3 : assert a==15 : a/=5 : assert a==3 : a&=2 : assert a==2 : a+=10 : assert a==12 : "
                "a-=3 : assert a==9 : a%=5 : assert a==4 : a|=1 : assert a==5 : a<<=3 : assert a==40 : "
                "a>>=1 : assert a==20 : nop";

#define AUTOTEST_OPERATORASSIGN2 "a=1 : a+=2 : assert a==3 : repeat 2 : a+=10 : rend : assert a==23: a=1 : a-=2 : assert a==-1 : repeat 2 : a-=10 : rend : assert a==-21: "
                "a=3 : a*=2 : assert a==6 : repeat 2 : a*=10 : rend : assert a==600: a=600 : a/=2 : assert a==300 : repeat 2 : a/=10 : rend : assert a==3 : nop";

#define AUTOTEST_OPERATORASSIGN3 "a=1 : a += 2 : assert a==3 : repeat 2 : a += 10 : rend : assert a==23: a=1 : a -= 2 : assert a==-1 : repeat 2 : a -= 10 : rend : assert a==-21: "
"a=3 : a *= 2 : assert a==6 : repeat 2 : a *= 10 : rend : assert a==600: a=600 : a /= 2 : assert a==300 : repeat 2 : a /= 10 : rend : assert a==3 : nop";


// x replaced by {x}
#define AUTOTEST_REPEAT3 "y=0: repeat 0: y+=1: rend: assert y==0: y=0: repeat 1: y+=1: rend: assert y==1:"
    "y=0: repeat 5: y+=1: rend: assert y==5: y=1: repeat 5,x: assert y=={x}: y+=1: rend: y=2: repeat 5,x,2,2: assert {x}==y:"
    "y+=2: rend: startingindex 5: y=5: repeat 2,x: assert {x}==y: y+=1: rend: startingindex 5,5: y=10: repeat 3,x,10: assert {x}==y: "
    "y+=5: rend: startingindex: y=1: repeat 2,x: assert {x}==y: y+=1: rend: nop ";


    // XXX a decision has to be taken. BANK semantic seems to be different in both assemblers
    #define AUTOTEST_SNASET "buildsna:bank 0:nop:"
    ":snaset Z80_AF,0x1234 :snaset Z80_A,0x11 :snaset Z80_F,0x11 :snaset Z80_BC,0x11 :snaset Z80_B,0x11 :snaset Z80_C,0x11 :snaset Z80_DE,0x11 :snaset Z80_D,0x11"
    ":snaset Z80_E,0x11 :snaset Z80_HL,0x11 :snaset Z80_H,0x11 :snaset Z80_L,0x11 :snaset Z80_R,0x11 :snaset Z80_I,0x11 :snaset Z80_IFF0,0x11 :snaset Z80_IFF1,0x11"
    ":snaset Z80_IX,0x11 :snaset Z80_IY,0x11 :snaset Z80_IXL,0x11 :snaset Z80_IXH,0x11 :snaset Z80_IYL,0x11 :snaset Z80_IYH,0x11 :snaset Z80_SP,0x11 :snaset Z80_PC,0x11"
    ":snaset Z80_IM,0x11 :snaset Z80_AFX,0x11 :snaset Z80_AX,0x11 :snaset Z80_FX,0x11 :snaset Z80_BCX,0x11 :snaset Z80_BX,0x11 :snaset Z80_CX,0x11 :snaset Z80_DEX,0x11"
    ":snaset Z80_DX,0x11 :snaset Z80_EX,0x11 :snaset Z80_HLX,0x11 :snaset Z80_HX,0x11 :snaset Z80_LX,0x11 :snaset FDD_MOTOR,0x11 :snaset FDD_TRACK,0x11 :snaset PRNT_DATA,0x11"
    ":snaset PPI_A,0x1 :snaset PPI_B,0x1 :snaset PPI_C,0x1 :snaset PPI_CTL,0x1 :snaset INT_NUM,0x1 :snaset INT_REQ,0x11 :snaset PSG_SEL,0x1 :snaset CRTC_SEL,0x1"
    ":snaset CRTC_TYPE,0x1 :snaset CRTC_HCC,0x11 :snaset CRTC_CLC,0x11 :snaset CRTC_RLC,0x11 :snaset CRTC_VAC,0x11 :snaset CRTC_VSWC,0x11 :snaset CRTC_HSWC,0x11"
    ":snaset CRTC_STATE,0x11 :snaset GA_VSC,0x11 :snaset GA_ISC,0x11 :snaset GA_PEN,0x11 :snaset GA_ROMCFG,0x11 :snaset GA_RAMCFG,0x11 :snaset ROM_UP,0x11"
    ":snaset CRTC_REG,0,0x11 :snaset CRTC_REG,1,0x11 :snaset CRTC_REG,16,0x11 :snaset PSG_REG,0,0x11 :snaset PSG_REG,5,0x11 :snaset PSG_REG,15,0x11: bank : nop";
    #define AUTOTEST_VIRGULE2 "print '5,,5':nop";
    // (void) added
    #define AUTOTEST_IFDEFMACRO	"macro test:nop:endm:ifndef test:error:else:test (void):endif:ifdef test:test (void):else:error:endif:nop";
    #define AUTOTEST_NOT 	"myvar=10:myvar=10+myvar:if 5!=3:else:print glop:endif:ifnot 5:print glop:else:endif:"
                        "ifnot 0:else:print glop:endif:if !(5):print glop:endif:if !(0):else:print glop:endif:"
                        "ya=!0:if ya==1:else:print glop:endif:if !5:print glop:endif:ya = 0:ya =! 0:if ya == 1:"
                        "else:print glop:endif:if ! 5:print glop:endif:if 1-!( !0 && !0):else:print glop:endif:nop" ;
    #define AUTOTEST_LABNUM 	"mavar=67:label{mavar}truc:ld hl,7+2*label{mavar}truc:mnt=1234567:lab2{mavar}{mnt}:"
    "ld de,lab2{mavar}{mnt}:lab3{mavar}{mnt}h:ld de,lab3{mavar}{mnt}h";
    #define AUTOTEST_EQUNUM		"mavar = 9:monlabel{mavar+5}truc:unalias{mavar+5}heu equ 50:autrelabel{unalias14heu}:ld hl,autrelabel50";
    #define AUTOTEST_TICKER		"repeat 2: ticker start, mc:out (0),a:out (c),a:out (c),h:out (c),0:ticker stop, mc:if mc!=15:ld hl,bite:else:nop:endif:rend";
    #define AUTOTEST_CHARSET
    "charset 'abcde',0:defb 'abcde':defb 'a','b','c','d','e':defb 'a',1*'b','c'*1,1*'d','e'*1:charset:"
                            "defb 'abcde':defb 'a','b','c','d','e':defb 'a',1*'b','c'*1,1*'d','e'*1" ;
    #define AUTOTEST_CHARSET2
    :chk(0x312):
    :len(16):
    "charset 97,97+26,0:defb 'roua':charset:charset 97,10:defb 'roua':charset 'o',5:defb 'roua':charset 'ou',6:defb 'roua'";

    #define AUTOTEST_LZSEGMENT	"org #100:debut:jr nz,zend:lz48:repeat 128:nop:rend:lzclose:jp zend:lz48:repeat 2:dec a:jr nz,@next:ld a,5:@next:jp debut:rend:"
                            "lzclose:zend";

#define AUTOTEST_MULTILZ	"bank: jr suivant: lz48: defs 100,#FF: lzclose: suivant jr lafin: lz49: defs 100,#FF: lzclose: lafin: bank: jr preums: "
                " preums jr suivant2: lzapu: defs 100,#FF: lzclose: suivant2 jr lafin2: lzexo: defs 100,#FF: lzclose: lafin2 ";

#define AUTOTEST_MULTILZORG " tabeul: defw script0: defw script1: defw script2: "
                "script0: org #4000,$: lz48: start1: jr end1: defs 100: djnz start1: end1: lzclose: "
                "org $: script1: org #4000,$: lz49: start2: jr end2: defs 100: djnz start2: end2: lzclose: "
                "org $: script2: org #4000,$: lz49: start3: jr end3: defs 100: djnz start3: end3: lzclose: "
                "org $: ei: ret ";

#define AUTOTEST_LZDEFERED	"lz48:defs 20:lzclose:defb $";
#define AUTOTEST_MACROPROX	" macro unemacro: nop: endm: global_label: ld hl, .table: .table";
#define AUTOTEST_LOCAPROX	"repeat 1: @label  nop: .prox   nop: @label2 nop: djnz @label.prox: rend";
#define AUTOTEST_FORMULA1	"a=5:b=2:assert int(a/b)==3:assert !a+!b==0:a=a*100:b=b*100:assert a*b==100000:ld hl,a*b-65536:a=123+-5*(-6/2)-50*2<<1";
#define AUTOTEST_LIMIT06	"org #FFFF : nop";
#define AUTOTEST_EMBEDDED_LABELS " disarkCounter = 0:MACRO dkps:PLY_AKG_DisarkPointerRegionStart_{disarkCounter}:ENDM"
        ":MACRO dkpe\nPLY_AKG_DisarkPointerRegionEnd_{disarkCounter}:\ndisarkCounter = disarkCounter + 1:ENDM:\ndkps\ndkpe\ndkps";

#define AUTOTEST_INTORAM1 " buildsna : bankset 0 :  org #3FF8 : defb 'roudoudou in da house' ";

#define AUTOTEST_DEFMOD "nbt=0: module preums: label1 nop: label3: label4: ifdef label1:nbt+=1:endif: ifdef label3:nbt+=1:endif:"
"ifdef label4:nbt+=1:endif: ifndef label5:nbt+=1:endif: module deuze: label1 nop: label3: label5: ifdef label1:nbt+=1:endif:"
"ifdef label3:nbt+=1:endif: ifndef label4:nbt+=1:endif: ifdef label5:nbt+=1:endif: assert nbt==8:"
"module grouik: plop: ifused plop : glop=1 : endif:assert glop==1";
}

// AUTOTEST_INSTRMUSTFAILED is not tested as expected by asm as we stop right after the first failure
import_rasm_failure! {
    #define AUTOTEST_BADINCLUDE "include 'truc\n .bin' \n nop nop";
    #define AUTOTEST_BADINCBIN "incl49 'truc\n .bin' \n nop nop";
    #define AUTOTEST_BADINCLUDE02 "read: ldir : cp ';'";
    #define AUTOTEST_SETINSIDE "ld hl,0=0xC9FB";
    #define AUTOTEST_INSTRMUSTFAILED "ld a,b,c:ldi a: ldir bc:exx hl,de:exx de:ex bc,hl:ex hl,bc:ex af,af:ex hl,hl:ex hl:exx hl: "
    "neg b:push b:push:pop:pop c:sub ix:add ix:add:sub:di 2:ei 3:ld i,c:ld r,e:rl:rr:rlca a:sla:sll:"
    "ldd e:lddr hl:adc ix:adc b,a:xor 12,13:xor b,1:xor:or 12,13:or b,1:or:and 12,13:and b,1:and:inc:dec";
    #define AUTOTEST_VIRGULE "defb 5,5,,5";
    #define AUTOTEST_REPEATKO	"repeat 5:nop";
    #define AUTOTEST_WHILEKO	"while 5:nop";
    #define AUTOTEST_SAVEINVALID1	"nop : save'gruik',20,-100";

#define AUTOTEST_SAVEINVALID2	"nop : save'gruik',-20,100";

#define AUTOTEST_SAVEINVALID3	"nop : save'gruik',40000,30000";

#define AUTOTEST_MACRO_CONF01   " nop: macro label1: nop: mend: macro label1: nop: mend:";
#define AUTOTEST_MACRO_CONF02   " label1=0: macro label1: nop: mend: nop:";
#define AUTOTEST_MACRO_CONF03   " label1 equ #100: macro label1: nop: mend: nop:";
#define AUTOTEST_MACRO_CONF04   " label1 nop: macro label1: nop: mend: nop";
#define AUTOTEST_LIMITKO	"limit #100:org #100:add ix,ix";
#define AUTOTEST_LIMIT03	"limit -1 : nop";
#define AUTOTEST_LIMIT04	"limit #10001 : nop";
#define AUTOTEST_LIMIT05	"org #FFFF : ldir";
#define AUTOTEST_LIMIT07	"org #ffff : Start:  equ $ :    di :     ld hl,#c9fb :  ld (#38),hl";
#define AUTOTEST_DELAYED_RUN "run _start:nop:_start nop";
#define AUTOTEST_INHIBITION	"if 0:ifused truc:ifnused glop:ifdef bidule:ifndef machin:ifnot 1:nop:endif:nop:else:nop:endif:endif:endif:endif:endif";
#define AUTOTEST_LZ4	"lz4:repeat 10:nop:rend:defb 'roudoudoudouoneatxkjhgfdskljhsdfglkhnopnopnopnop':lzclose";
#define AUTOTEST_LIMITOK "org #100:limit #102:nop:limit #103:ld a,0:protect #105,#107:limit #108:xor a:org $+3:inc a" ;

}

fn assemble_success(code: &str, verify: VerifyOutput) {
    test_assemble(code, true, verify)
}

fn assemble_failure(code: &str, verify: VerifyOutput) {
    test_assemble(code, false, verify)
}

fn test_assemble(code: &str, success: bool, verify: VerifyOutput) {
    let input_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let input_fname = input_file.path().as_os_str().to_str().unwrap();
    std::fs::write(input_fname, code).unwrap();

    let output_file = tempfile::NamedTempFile::new().expect("Unable to build temporary file");
    let output_fname = output_file.path().as_os_str().to_str().unwrap();

    let res = Command::new("../target/debug/basm")
        .args(["-I", "tests/asm/", "-i", input_fname, "-o", output_fname])
        .output()
        .expect("Unable to launch basm");

    if success && res.status.success() {
        let res = std::fs::read(output_fname).unwrap();

        if let Some(chk) = verify.chk {
            let sum: usize = res.iter().map(|b| *b as usize).sum();
            assert_eq!(chk, sum);
        }

        if let Some(size) = verify.size {
            assert_eq!(size, res.len());
        }

        for (idx, byte) in verify.opcodes {
            assert_eq!(byte, res[idx]);
        }
    }

    if success && !res.status.success() {
        panic!(
            "Failure to assemble right code.{}",
            String::from_utf8_lossy(&res.stderr)
        );
    }

    if !success && res.status.success() {
        panic!(
            "Succeed to assemble wrong code{}",
            String::from_utf8_lossy(&res.stderr)
        );
    }
}
