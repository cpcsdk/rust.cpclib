; Simpler test to understand the issue
    org 0x4000

    ; Test simple struct with single byte fields
    struct TEST1
        field1 db
        field2 db
        field3 db
    endstruct
    
    assert TEST1 == 3, "TEST1 should be 3 bytes"
    assert TEST1.field1 == 0
    assert TEST1.field2 == 1
    assert TEST1.field3 == 2
    
    ; Test struct with db and default values
    struct TEST2
        field1 db 1
        field2 db 2
        field3 db 3
    endstruct
    
    assert TEST2 == 3, "TEST2 should be 3 bytes"
    assert TEST2.field1 == 0
    assert TEST2.field2 == 1
    assert TEST2.field3 == 2
    
    ; Test struct with opcode() as default value
    ; struct TEST3
    ;     field1 db
    ;     field2 db
    ;     field3 db opcode(nop)
    ; endstruct
    
    ; print "TEST3 size:", TEST3
    ; print "TEST3.field1:", TEST3.field1
    ; print "TEST3.field2:", TEST3.field2
    ; print "TEST3.field3:", TEST3.field3
    
    ; assert TEST3 == 3, "TEST3 should be 3 bytes"
    ; assert TEST3.field3 == 2, "field3 should be at offset 2"
    
    ; Test with list_get(opcode(...))
    struct TEST4
        field1 db
        field2 db
        field3 db list_get(opcode(out (c), d), 1)
    endstruct
    
    print "TEST4 size:", TEST4
    print "TEST4.field1:", TEST4.field1
    print "TEST4.field2:", TEST4.field2
    print "TEST4.field3:", TEST4.field3
    
    assert TEST4 == 3, "TEST4 should be 3 bytes"
    assert TEST4.field3 == 2, "field3 should be at offset 2"
