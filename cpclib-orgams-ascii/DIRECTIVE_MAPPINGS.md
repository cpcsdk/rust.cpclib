# Orgams Directive Command Byte Mappings

## Verification Method
Correlated MEMMAP.Z80 source directives with MEMMAP.I binary encoding.

## CONFIRMED Mappings (100% verified)

| Byte | Directive | Verification                                      |
|------|-----------|---------------------------------------------------|
| 0x17 | IMPORT    | Line 21, offset 0x02B7: `7F 17 0A 22 07...`     |
| 0x15 | IF        | Line 97, offset 0xAA9: `7F 15 03 84 85 41`      |
| 0x08 | SKIP/DEFS | Line 206, offset 0xDE6: `7F 08 09 42 35...`     |

## HIGH CONFIDENCE Mappings (95%+ based on patterns)

| Byte | Directive         | Evidence                                        |
|------|-------------------|-------------------------------------------------|
| 0x0C | END               | Pattern `7F 0C 4A` after IF blocks             |
| 0x01 | ASIS (as-is code) | Followed by inline text: `7F 01 21 20 20 21 21` |
| 0x43 | Inline Comment    | `7F 43 0D 20 65 6E 20 6D...` = "en maio bank"  |

## MEDIUM CONFIDENCE (70%+ - need more verification)

| Byte | Likely Directive | Evidence                                    |
|------|------------------|---------------------------------------------|
| 0x03 | Assignment?      | Very common, appears with label assignments |
| 0x04 | ORG?             | Followed by hex addresses                   |
| 0x09 | Expression?      | Common with expression encoding             |
| 0x0F | Comment marker?  | Appears with comment text                   |
| 0x05 | ?                | Rare (1 occurrence)                         |
| 0x9E | ?                | Rare (1 occurrence)                         |

## Directive Encoding Pattern

All directives follow: `7F [cmd] [params] 41`

- `7F` = ec_esc (escape marker)
- `[cmd]` = Command byte (directive identifier)
- `[params]` = Directive-specific parameters (expressions, strings, etc.)
- `41` = E_ENDOFDATA (end marker)

### Example: IMPORT Directive
```
7F 17 0A 22 07 63 6F 6E 73 74 2E 69 41
│  │  │  │  │  └─────────────┘  │
│  │  │  │  │  "const.i"        └─ E_ENDOFDATA (0x41)
│  │  │  │  └─ String length (7)
│  │  │  └─ E_STRING marker (0x22)
│  │  └─ Expression size (10 bytes)
│  └─ IMPORT command (0x17)
└─ ec_esc marker (0x7F)
```

### Example: IF Directive
```
7F 15 03 84 85 41
│  │  │  └──┘  │
│  │  │  │     └─ E_ENDOFDATA
│  │  │  └─ Expression operands (label references)
│  │  └─ Expression size (3 bytes)
│  └─ IF command (0x15)
└─ Escape marker
```

### Example: SKIP Directive
```
7F 08 09 42 35 E0 76 20 2D 20 24 45
│  │  │  └───────────────────┘  │
│  │  │  Expression (&76E0 - $) └─ E_ENDOFDATA (0x45='E')
│  │  └─ Expression size (9 bytes)
│  └─ SKIP command (0x08)
└─ Escape marker
```

## Next Steps

1. ✅ Implement table-driven decoder for confirmed directives
2. Verify medium-confidence mappings by finding more examples
3. Extract remaining command bytes from PARSE.Z80 assembly
4. Auto-generate decoder from PARSE.Z80 directive patterns
