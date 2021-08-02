    org 0x1234

        align 256
        assert $ == 0x1300

        align 256
        assert $ == 0x1300

        nop
        
        align 128, 3
        assert $ == 0x1300 + 128