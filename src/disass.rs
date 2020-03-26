use crate::assembler::tokens::*;
use crate::assembler::builder::*;
use std::convert::TryFrom;


// Tables stolen from https://github.com/jeromelesaux/dsk/blob/master/desassembly.go
// Note that these table do not all contain 256 values; I have added missing ones without checking if they are at the right place
const TABINSTRFDCB:[&'static str;256]  = [
	"", "", "", "", "", "", "RLC (IY+nn)", "",
	"", "", "", "", "", "", "RRC (IY+nn)", "",
	"", "", "", "", "", "", "RL (IY+nn)", "",
	"", "", "", "", "", "", "RR (IY+nn)", "",
	"", "", "", "", "", "", "SLA (IY+nn)", "",
	"", "", "", "", "", "", "SRA (IY+nn)", "",
	"", "", "", "", "", "", "SLL (IY+nn)", "",
	"", "", "", "", "", "", "SRL (IY+nn)", "",
	"", "", "", "", "", "", "BIT 0,(IY+nn)", "",
	"", "", "", "", "", "", "BIT 1,(IY+nn)", "",
	"", "", "", "", "", "", "BIT 2,(IY+nn)", "",
	"", "", "", "", "", "", "BIT 3,(IY+nn)", "",
	"", "", "", "", "", "", "BIT 4,(IY+nn)", "",
	"", "", "", "", "", "", "BIT 5,(IY+nn)", "",
	"", "", "", "", "", "", "BIT 6,(IY+nn)", "",
	"", "", "", "", "", "", "BIT 7,(IY+nn)", "",
	"", "", "", "", "", "", "RES 0,(IY+nn)", "",
	"", "", "", "", "", "", "RES 1,(IY+nn)", "",
	"", "", "", "", "", "", "RES 2,(IY+nn)", "",
	"", "", "", "", "", "", "RES 3,(IY+nn)", "",
	"", "", "", "", "", "", "RES 4,(IY+nn)", "",
	"", "", "", "", "", "", "RES 5,(IY+nn)", "",
	"", "", "", "", "", "", "RES 6,(IY+nn)", "",
	"", "", "", "", "", "", "RES 7,(IY+nn)", "",
	"", "", "", "", "", "", "SET 0,(IY+nn)", "",
	"", "", "", "", "", "", "SET 1,(IY+nn)", "",
	"", "", "", "", "", "", "SET 2,(IY+nn)", "",
	"", "", "", "", "", "", "SET 3,(IY+nn)", "",
	"", "", "", "", "", "", "SET 4,(IY+nn)", "",
	"", "", "", "", "", "", "SET 5,(IY+nn)", "",
	"", "", "", "", "", "", "SET 6,(IY+nn)", "",
	"", "", "", "", "", "", "SET 7,(IY+nn)", "",
];


const TABINSTRDDCB:[&'static str;256]  = [
	"", "", "", "", "", "", "RLC (IX+nn)", "",
	"", "", "", "", "", "", "RRC (IX+nn)", "",
	"", "", "", "", "", "", "RL (IX+nn)", "",
	"", "", "", "", "", "", "RR (IX+nn)", "",
	"", "", "", "", "", "", "SLA (IX+nn)", "",
	"", "", "", "", "", "", "SRA (IX+nn)", "",
	"", "", "", "", "", "", "SLL (IX+nn)", "",
	"", "", "", "", "", "", "SRL (IX+nn)", "",
	"", "", "", "", "", "", "BIT 0,(IX+nn)", "",
	"", "", "", "", "", "", "BIT 1,(IX+nn)", "",
	"", "", "", "", "", "", "BIT 2,(IX+nn)", "",
	"", "", "", "", "", "", "BIT 3,(IX+nn)", "",
	"", "", "", "", "", "", "BIT 4,(IX+nn)", "",
	"", "", "", "", "", "", "BIT 5,(IX+nn)", "",
	"", "", "", "", "", "", "BIT 6,(IX+nn)", "",
	"", "", "", "", "", "", "BIT 7,(IX+nn)", "",
	"", "", "", "", "", "", "RES 0,(IX+nn)", "",
	"", "", "", "", "", "", "RES 1,(IX+nn)", "",
	"", "", "", "", "", "", "RES 2,(IX+nn)", "",
	"", "", "", "", "", "", "RES 3,(IX+nn)", "",
	"", "", "", "", "", "", "RES 4,(IX+nn)", "",
	"", "", "", "", "", "", "RES 5,(IX+nn)", "",
	"", "", "", "", "", "", "RES 6,(IX+nn)", "",
	"", "", "", "", "", "", "RES 7,(IX+nn)", "",
	"", "", "", "", "", "", "SET 0,(IX+nn)", "",
	"", "", "", "", "", "", "SET 1,(IX+nn)", "",
	"", "", "", "", "", "", "SET 2,(IX+nn)", "",
	"", "", "", "", "", "", "SET 3,(IX+nn)", "",
	"", "", "", "", "", "", "SET 4,(IX+nn)", "",
	"", "", "", "", "", "", "SET 5,(IX+nn)", "",
	"", "", "", "", "", "", "SET 6,(IX+nn)", "",
	"", "", "", "", "", "", "SET 7,(IX+nn)", "",
];

const TABINSTRCB:[&'static str;256]  = [
	"RLC B", "RLC C", "RLC D", "RLC E",
	"RLC H", "RLC L", "RLC (HL)", "RLC A",
	"RRC B", "RRC C", "RRC D", "RRC E",
	"RRC H", "RRC L", "RRC (HL)", "RRC A",
	"RL B", "RL C", "RL D", "RL E",
	"RL H", "RL L", "RL (HL)", "RL A",
	"RR B", "RR C", "RR D", "RR E",
	"RR H", "RR L", "RR (HL)", "RR A",
	"SLA B", "SLA C", "SLA D", "SLA E",
	"SLA H", "SLA L", "SLA (HL)", "SLA A",
	"SRA B", "SRA C", "SRA D", "SRA E",
	"SRA H", "SRA L", "SRA (HL)", "SRA A",
	"SLL B", "SLL C", "SLL D", "SLL E",
	"SLL H", "SLL L", "SLL (HL)", "SLL A",
	"SRL B", "SRL C", "SRL D", "SRL E",
	"SRL H", "SRL L", "SRL (HL)", "SRL A",
	"BIT 0,B", "BIT 0,C", "BIT 0,D", "BIT 0,E",
	"BIT 0,H", "BIT 0,L", "BIT 0,(HL)", "BIT 0,A",
	"BIT 1,B", "BIT 1,C", "BIT 1,D", "BIT 1,E",
	"BIT 1,H", "BIT 1,L", "BIT 1,(HL)", "BIT 1,A",
	"BIT 2,B", "BIT 2,C", "BIT 2,D", "BIT 2,E",
	"BIT 2,H", "BIT 2,L", "BIT 2,(HL)", "BIT 2,A",
	"BIT 3,B", "BIT 3,C", "BIT 3,D", "BIT 3,E",
	"BIT 3,H", "BIT 3,L", "BIT 3,(HL)", "BIT 3,A",
	"BIT 4,B", "BIT 4,C", "BIT 4,D", "BIT 4,E",
	"BIT 4,H", "BIT 4,L", "BIT 4,(HL)", "BIT 4,A",
	"BIT 5,B", "BIT 5,C", "BIT 5,D", "BIT 5,E",
	"BIT 5,H", "BIT 5,L", "BIT 5,(HL)", "BIT 5,A",
	"BIT 6,B", "BIT 6,C", "BIT 6,D", "BIT 6,E",
	"BIT 6,H", "BIT 6,L", "BIT 6,(HL)", "BIT 6,A",
	"BIT 7,B", "BIT 7,C", "BIT 7,D", "BIT 7,E",
	"BIT 7,H", "BIT 7,L", "BIT 7,(HL)", "BIT 7,A",
	"RES 0,B", "RES 0,C", "RES 0,D", "RES 0,E",
	"RES 0,H", "RES 0,L", "RES 0,(HL)", "RES 0,A",
	"RES 1,B", "RES 1,C", "RES 1,D", "RES 1,E",
	"RES 1,H", "RES 1,L", "RES 1,(HL)", "RES 1,A",
	"RES 2,B", "RES 2,C", "RES 2,D", "RES 2,E",
	"RES 2,H", "RES 2,L", "RES 2,(HL)", "RES 2,A",
	"RES 3,B", "RES 3,C", "RES 3,D", "RES 3,E",
	"RES 3,H", "RES 3,L", "RES 3,(HL)", "RES 3,A",
	"RES 4,B", "RES 4,C", "RES 4,D", "RES 4,E",
	"RES 4,H", "RES 4,L", "RES 4,(HL)", "RES 4,A",
	"RES 5,B", "RES 5,C", "RES 5,D", "RES 5,E",
	"RES 5,H", "RES 5,L", "RES 5,(HL)", "RES 5,A",
	"RES 6,B", "RES 6,C", "RES 6,D", "RES 6,E",
	"RES 6,H", "RES 6,L", "RES 6,(HL)", "RES 6,A",
	"RES 7,B", "RES 7,C", "RES 7,D", "RES 7,E",
	"RES 7,H", "RES 7,L", "RES 7,(HL)", "RES 7,A",
	"SET 0,B", "SET 0,C", "SET 0,D", "SET 0,E",
	"SET 0,H", "SET 0,L", "SET 0,(HL)", "SET 0,A",
	"SET 1,B", "SET 1,C", "SET 1,D", "SET 1,E",
	"SET 1,H", "SET 1,L", "SET 1,(HL)", "SET 1,A",
	"SET 2,B", "SET 2,C", "SET 2,D", "SET 2,E",
	"SET 2,H", "SET 2,L", "SET 2,(HL)", "SET 2,A",
	"SET 3,B", "SET 3,C", "SET 3,D", "SET 3,E",
	"SET 3,H", "SET 3,L", "SET 3,(HL)", "SET 3,A",
	"SET 4,B", "SET 4,C", "SET 4,D", "SET 4,E",
	"SET 4,H", "SET 4,L", "SET 4,(HL)", "SET 4,A",
	"SET 5,B", "SET 5,C", "SET 5,D", "SET 5,E",
	"SET 5,H", "SET 5,L", "SET 5,(HL)", "SET 5,A",
	"SET 6,B", "SET 6,C", "SET 6,D", "SET 6,E",
	"SET 6,H", "SET 6,L", "SET 6,(HL)", "SET 6,A",
	"SET 7,B", "SET 7,C", "SET 7,D", "SET 7,E",
	"SET 7,H", "SET 7,L", "SET 7,(HL)", "SET 7,A",
];



const TABINSTRED:[&'static str;256] = [
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"IN B,(C)", "OUT (C),B", "SBC HL,BC", "LD (nnnn),BC",
	"NEG", "RETN", "IM 0", "LD I,A",
	"IN C,(C)", "OUT (C),C", "ADC HL,BC", "LD BC,(nnnn)",
	"", "RETI", "", "LD R,A",
	"IN D,(C)", "OUT (C),D", "SBC HL,DE", "LD (nnnn),DE",
	"", "", "IM 1", "LD A,I",
	"IN E,(C)", "OUT (C),E", "ADC HL,DE", "LD DE,(nnnn)",
	"", "", "IM 2", "LD A,R",
	"IN H,(C)", "OUT (C),H", "SBC HL,HL", "",
	"", "", "", "RRD",
	"IN L,(C)", "OUT (C),L", "ADC HL,HL", "",
	"", "", "", "RLD",
	"", "OUT (C),0", "SBC HL,SP", "LD (nnnn),SP",
	"", "", "", "",
	"IN A,(C)", "OUT (C),A", "ADC HL,SP", "LD SP,(nnnn)",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"LDI", "CPI", "INI", "OUTI",
	"", "", "", "",
	"LDD", "CPD", "IND", "OUTD",
	"", "", "", "",
	"LDIR", "CPIR", "INIR", "OTIR",
	"", "", "", "",
	"LDDR", "CPDR", "INDR", "OTDR",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
];

const TABINSTRDD:[&'static str;256]  = [
	"", "", "", "",
	"", "", "", "",
	"", "ADD IX,BC", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "ADD IX,DE", "", "",
	"", "", "", "",
	"", "LD IX,nnnn", "LD (nnnn),IX", "INC IX",
	"INC IXh", "DEC IXh", "LD IXh,nn", "",
	"", "ADD IX,HL", "LD IX,(nnnn)", "DEC IX",
	"INC IXl", "DEC IXl", "LD IXl,nn", "",
	"", "", "", "",
	"INC (IX+nn)", "DEC (IX+nn)", "LD (IX+nn),nn", "",
	"", "ADD IX,SP", "", "",
	"", "", "", "",
	"", "", "", "",
	"LD B,IXh", "LD B,IXl", "LD B,(IX+nn)", "",
	"", "", "", "",
	"LD C,IXh", "LD C,IXl", "LD C,(IX+nn)", "",
	"", "", "", "",
	"LD D,IXh", "LD D,IXl", "LD D,(IX+nn)", "",
	"", "", "", "",
	"LD E,IXh", "LD E,IXl", "LD E,(IX+nn)", "",
	"LD IXh,B", "LD IXh,C", "LD IXh,D", "LD IXh,E",
	"LD IXh,IXh", "LD IXh,IXl", "LD H,(IX+nn)", "LD IXh,A",
	"LD IXl,B", "LD IXl,C", "LD IXl,D", "LD IXl,E",
	"LD IXl,IXh", "LD IXl,IXl", "LD L,(IX+nn)", "LD IXl,A",
	"LD (IX+nn),B", "LD (IX+nn),C", "LD (IX+nn),D", "LD (IX+nn),E",
	"LD (IX+nn),H", "LD (IX+nn),L", "", "LD (IX+nn),A",
	"", "", "", "",
	"LD A,IXh", "LD A,IXl", "LD A,(IX+nn)", "",
	"", "", "", "",
	"ADD A,IXh", "ADD A,IXl", "ADD A,(IX+nn)", "",
	"", "", "", "",
	"ADC A,IXh", "ADC A,IXl", "ADC A,(IX+nn)", "",
	"", "", "", "",
	"SUB IXh", "SUB IXl", "SUB (IX+nn)", "",
	"", "", "", "",
	"SBC A,IXh", "SBC A,IXl", "SBC A,(IX+nn)", "",
	"", "", "", "",
	"AND IXh", "AND IXl", "AND (IX+nn)", "",
	"", "", "", "",
	"XOR IXh", "XOR IXl", "XOR (IX+nn)", "",
	"", "", "", "",
	"OR IXh", "OR IXl", "OR (IX+nn)", "",
	"", "", "", "",
	"CP IXh", "CP IXl", "CP (IX+nn)", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "POP IX", "", "EX (SP),IX",
	"", "PUSH IX", "", "",
	"", "JP (IX)", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "", "", "",
	"", "LD SP,IX", "", "",
	"", "", "", "",
];



const TABINSTRFD:[&'static str;256]  = [
	"", "", "", "", "", "", "", "",
	"", "ADD IY,BC", "", "", "", "", "", "",
	"", "", "", "", "", "", "", "",
	"", "ADD IY,DE", "", "", "", "", "", "",
	"", "LD IY,nnnn", "LD (nnnn),IY", "INC IY", "INC IYh", "DEC IYh", "LD IYh,nn", "",
	"", "ADD IY,HL", "LD IY,(nnnn)", "DEC IY", "INC IYl", "DEC IYl", "LD IYl,nn", "",
	"", "", "", "", "INC (IY+nn)", "DEC (IY+nn)", "LD (IY+nn),nn", "",
	"", "ADD IY,SP", "", "", "", "", "", "",
	"", "", "", "", "LD B,IYh", "LD B,IYl", "LD B,(IY+nn)", "",
	"", "", "", "", "LD C,IYh", "LD C,IYl", "LD C,(IY+nn)", "",
	"", "", "", "", "LD D,IYh", "LD D,IYl", "LD D,(IY+nn)", "",
	"", "", "", "", "LD E,IYh", "LD E,IYl", "LD E,(IY+nn)", "",
	"LD IYh,B", "LD IYh,C", "LD IYh,D", "LD IYh,E", "LD IYh,IYh", "LD IYh,IYl", "LD H,(IY+nn)", "LD IYh,A",
	"LD IYl,B", "LD IYl,C", "LD IYl,D", "LD IYl,E", "LD IYl,IYh", "LD IYl,IYl", "LD L,(IY+nn)", "LD IYl,A",
	"LD (IY+nn),B", "LD (IY+nn),C", "LD (IY+nn),D", "LD (IY+nn),E", "LD (IY+nn),H", "LD (IY+nn),L", "", "LD (IY+nn),A",
	"", "", "", "", "LD A,IYh", "LD A,IYl", "LD A,(IY+nn)", "",
	"", "", "", "", "ADD A,IYh", "ADD A,IYl", "ADD A,(IY+nn)", "",
	"", "", "", "", "ADC A,IYh", "ADC A,IYl", "ADC A,(IY+nn)", "",
	"", "", "", "", "SUB IYh", "SUB IYl", "SUB (IY+nn)", "",
	"", "", "", "", "SBC A,IYh", "SBC A,IYl", "SBC A,(IY+nn)", "",
	"", "", "", "", "AND IYh", "AND IYl", "AND (IY+nn)", "",
	"", "", "", "", "XOR IYh", "XOR IYl", "XOR (IY+nn)", "",
	"", "", "", "", "OR IYh", "OR IYl", "OR (IY+nn)", "",
	"", "", "", "", "CP IYh", "CP IYl", "CP (IY+nn)", "",
	"", "", "", "", "", "", "", "",
	"", "", "", "", "", "", "", "",
	"", "", "", "", "", "", "", "",
	"", "", "", "", "", "", "", "",
	"", "POP IY", "", "EX (SP),IY", "", "PUSH IY", "", "",
	"", "JP (IY)", "", "", "", "", "", "",
	"", "", "", "", "", "", "", "",
	"", "LD SP,IY", "", "", "", "", "", "",
];



const TABINSTR:[&'static str;256]  = [
	"NOP", "LD BC,nnnn", "LD (BC),A", "INC BC",
	"INC B", "DEC B", "LD B,nn", "RLCA",
	"EX AF,AF", "ADD HL,BC", "LD A,(BC)", "DEC BC",
	"INC C", "DEC C", "LD C,nn", "RRCA",
	"DJNZ eeee", "LD DE,nnnn", "LD (DE),A", "INC DE",
	"INC D", "DEC D", "LD D,nn", "RLA",
	"JR eeee", "ADD HL,DE", "LD A,(DE)", "DEC DE",
	"INC E", "DEC E", "LD E,nn", "RRA",
	"JR NZ,eeee", "LD HL,nnnn", "LD (nnnn),HL", "INC HL",
	"INC H", "DEC H", "LD H,nn", "DAA",
	"JR Z,eeee", "ADD HL,HL", "LD HL,(nnnn)", "DEC HL",
	"INC L", "DEC L", "LD L,nn", "CPL",
	"JR NC,eeee", "LD SP,nnnn", "LD (nnnn),A", "INC SP",
	"INC (HL)", "DEC (HL)", "LD (HL),nn", "SCF",
	"JR C,eeee", "ADD HL,SP", "LD A,(nnnn)", "DEC SP",
	"INC A", "DEC A", "LD A,nn", "CCF",
	"LD B,B", "LD B,C", "LD B,D", "LD B,E",
	"LD B,H", "LD B,L", "LD B,(HL)", "LD B,A",
	"LD C,B", "LD C,C", "LD C,D", "LD C,E",
	"LD C,H", "LD C,L", "LD C,(HL)", "LD C,A",
	"LD D,B", "LD D,C", "LD D,D", "LD D,E",
	"LD D,H", "LD D,L", "LD D,(HL)", "LD D,A",
	"LD E,B", "LD E,C", "LD E,D", "LD E,E",
	"LD E,H", "LD E,L", "LD E,(HL)", "LD E,A",
	"LD H,B", "LD H,C", "LD H,D", "LD H,E",
	"LD H,H", "LD H,L", "LD H,(HL)", "LD H,A",
	"LD L,B", "LD L,C", "LD L,D", "LD L,E",
	"LD L,H", "LD L,L", "LD L,(HL)", "LD L,A",
	"LD (HL),B", "LD (HL),C", "LD (HL),D", "LD (HL),E",
	"LD (HL),H", "LD (HL),L", "HALT", "LD (HL),A",
	"LD A,B", "LD A,C", "LD A,D", "LD A,E",
	"LD A,H", "LD A,L", "LD A,(HL)", "LD A,A",
	"ADD A,B", "ADD A,C", "ADD A,D", "ADD A,E",
	"ADD A,H", "ADD A,L", "ADD A,(HL)", "ADD A,A",
	"ADC A,B", "ADC A,C", "ADC A,D", "ADC A,E",
	"ADC A,H", "ADC A,L", "ADC A,(HL)", "ADC A,A",
	"SUB B", "SUB C", "SUB D", "SUB E",
	"SUB H", "SUB L", "SUB (HL)", "SUB A",
	"SBC A,B", "SBC A,C", "SBC A,D", "SBC A,E",
	"SBC A,H", "SBC A,L", "SBC A,(HL)", "SBC A,A",
	"AND B", "AND C", "AND D", "AND E",
	"AND H", "AND L", "AND (HL)", "AND A",
	"XOR B", "XOR C", "XOR D", "XOR E",
	"XOR H", "XOR L", "XOR (HL)", "XOR A",
	"OR B", "OR C", "OR D", "OR E",
	"OR H", "OR L", "OR (HL)", "OR A",
	"CP B", "CP C", "CP D", "CP E",
	"CP H", "CP L", "CP (HL)", "CP A",
	"RET NZ", "POP BC", "JP NZ,nnnn", "JP nnnn",
	"CALL NZ,nnnn", "PUSH BC", "ADD A,nn", "RST 00",
	"RET Z", "RET", "JP Z,nnnn", "",
	"CALL Z,nnnn", "CALL nnnn", "ADC A,nn", "RST 08",
	"RET NC", "POP DE", "JP NC,nnnn", "OUT (nn),A",
	"CALL NC,nnnn", "PUSH DE", "SUB nn", "RST 10",
	"RET C", "EXX", "JP C,nnnn", "IN A,(nn)",
	"CALL C,nnnn", "", "SBC A,nn", "RST 18",
	"RET PE", "POP HL", "JP PE,nnnn", "EX (SP),HL",
	"CALL PE,nnnn", "PUSH HL", "AND nn", "RST 20",
	"RET PO", "JP (HL)", "JP PO,nnnn", "EX DE,HL",
	"CALL PO,nnnn", "", "XOR nn", "RST 28",
	"RET P", "POP AF", "JP P,nnnn", "DI",
	"CALL P,nnnn", "PUSH AF", "OR nn", "RST 30",
	"RET M", "LD SP,HL", "JP M,nnnn", "EI",
	"CALL M,nnnn", "", "CP nn", "RST 38",
];



/// Generate a listing from the list of bytes. An error is generated if it is impossible to disassemble the flux
/// TODO really implement it
pub fn disassemble(bytes: &[u8]) -> Result<Listing, String> {

    // Generate a listing that contains the current token followed by tokens obtaines from remaining bytes
    let continue_disassembling = |token: Token, bytes: &[u8]| {
        disassemble(bytes).and_then(|rest|{
            // TODO -- check if it is possible to improve this aspect
            let mut lst = Listing::new();
            lst.push(token);
            lst.extend_from_slice(&rest);
            Ok(lst)
        })
    };


    match bytes {
        // Nothing to disassemble
        [] => {
            Ok(Listing::new())
        },

        // Current mnemonic is nop
        [0, rest @ ..] => {
            continue_disassembling(nop(), rest)           
        },

        [0xFD, 0xCB, param, opcode, rest @ ..] => {
            let token = disassemble_with_one_argument(*opcode, *param, &TABINSTRFDCB)?;
            continue_disassembling(token, rest)           
        },

        // TODO this case is buggy and needs  to be redone to tkae into acocunt the position of the argument
        [0xDD, 0xCB, param, opcode, rest @ ..] => {
            let token = disassemble_with_one_argument(*opcode, *param, &TABINSTRDDCB)?;
            continue_disassembling(token, rest)           
        },

        [0xCB, ref opcode, rest @ ..] => {
            let token = disassemble_without_argument(*opcode, &TABINSTRCB)?;
            continue_disassembling(token, rest)
        },

        [0xED, ref opcode, rest @ ..] => {
            let (token, rest) =  disassemble_with_potential_argument(*opcode, &TABINSTRED, rest)?;
            continue_disassembling(token, rest)
        },

        
        [0xDD, ref opcode, rest @ ..] => {
            let (token, rest) =  disassemble_with_potential_argument(*opcode, &TABINSTRDD, rest)?;
            continue_disassembling(token, rest)
        },

        [0xFD, ref opcode, rest @ ..] => {
            let (token, rest) =  disassemble_with_potential_argument(*opcode, &TABINSTRFD, rest)?;
            continue_disassembling(token, rest)
        },

        [ref opcode, rest @ ..] => {
            let (token, rest) =  disassemble_with_potential_argument(*opcode, &TABINSTR, rest)?;
            continue_disassembling(token, rest)
        }
    }

}

/// Manage the disassembling of the current instraction. However this instruction may need an argument.
/// For this reason the byte stream is provided to collect this argument if needed
fn disassemble_with_potential_argument<'stream>(opcode: u8, lut: &[&'static str;256], bytes: &'stream [u8]) -> Result<(Token, &'stream [u8]), String> {
    let representation:&'static str = lut[opcode as usize];

    // get the first argument if any
    let (representation, bytes) = if representation.contains("nnnn") {
        let word = bytes[0] as u16 + 256*(bytes[1] as u16);
        let representation = representation.replacen("nnnn", &format!("{:#03x}", word), 1);
        (representation, &bytes[2..])
    }
    else if representation.contains("nn") {
        let byte = bytes[0];
        let representation = representation.replacen("nn", &format!("{:#01x}", byte), 1);
        (representation.to_owned(), &bytes[1 ..])
    }
    else {
        (representation.to_owned(), bytes)
    };

    // get the second argument if any
    let (representation, bytes) = if representation.contains("nn") {
        let byte = bytes[0];
        let representation = representation.replacen("nn", &format!("{:#01x}", byte), 1);
        (representation, &bytes[1 ..])      
    }
    else {
        (representation, bytes)
    };

    Ok((string_to_token(&representation)?, bytes))
}

/// The 8bits argument has already been read
fn disassemble_with_one_argument(opcode: u8, argument: u8, lut: &[&'static str; 256]) -> Result<Token, String> {
    let representation:&'static str = lut[opcode as usize];
    let representation = representation.replacen("nn", &format!("{:#01x}", argument), 1);
    string_to_token(&representation)
}

/// No argument is expected
fn disassemble_without_argument(opcode: u8, lut: &[&'static str; 256]) -> Result<Token, String> {
    let representation:&'static str = lut[opcode as usize];
    string_to_token(&representation)
}

fn string_to_token(representation: &str) -> Result<Token, String> {
    Token::try_from(["\t", &representation].join(""))
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn disass_simple_instruction() {
        assert_eq!("PUSH HL", disassemble(&[0xe5]).unwrap().to_string().trim());
        assert_eq!("RES 0x3, E", disassemble(&[0xcb, 0x9b]).unwrap().to_string().trim());
    }

    #[test]
    fn disass_instruction_with_arg() {
        assert_eq!("CALL NZ, 0x123", disassemble(&[0xc4, 0x23, 0x01]).unwrap().to_string().trim());
        assert_eq!("LD IX, (0x4321)", disassemble(&[0xdd, 0x2a, 0x21, 0x43]).unwrap().to_string().trim());
        assert_eq!("LD (IX + 0x21), 0x43", disassemble(&[0xdd, 0x36, 0x21, 0x43]).unwrap().to_string().trim());
        assert_eq!("BIT 0x6, (IX + 0x1)", disassemble(&[0xdd, 0xcb, 0x01, 0x76]).unwrap().to_string().trim());
    }

    #[test]
    fn disass_unknwon_opcode(){
        assert!(disassemble(&[0xfd, 0x00]).is_err());
    }
}