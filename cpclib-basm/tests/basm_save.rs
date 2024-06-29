use cpclib_basm::*;

/// ! This test file has been created to track some wrong memory handling when saving data with banksets

#[test]
fn bankset_check_save() {
    let args_parser = build_args_parser();
    let args = args_parser.get_matches_from(&["basm", "-I", "tests/asm/", "good_bankset.asm"]);
    let (env, _) = process(&args).expect("Unable to assemble the file");

    let sna = env.sna();
    dbg!(sna.memory_size_header(), sna.memory_dump().len());
    assert_eq!(sna.memory_size_header(), 128);
    let mem = sna.memory_dump();

    static data_0_0: [u8; 4] = [1, 2, 3, 4];
    static data_0_1: [u8; 4] = [5, 6, 7, 8];
    static data_0_2: [u8; 4] = [9, 10, 11, 12];
    static data_0_3: [u8; 4] = [13, 14, 15, 16];

    static data_1_0: [u8; 4] = [10, 20, 30, 40];
    static data_1_1: [u8; 4] = [50, 60, 70, 80];
    static data_1_2: [u8; 4] = [90, 100, 110, 120];
    static data_1_3: [u8; 4] = [130, 140, 150, 160];

    // check the content of the snapshot
    assert_eq!(&mem[0x0000..(0x0000 + 4)], &data_0_0);

    assert_eq!(&mem[0x4000..(0x4000 + 4)], &data_0_1);

    assert_eq!(&mem[0x8000..(0x8000 + 4)], &data_0_2);

    assert_eq!(&mem[0xC000..(0xC000 + 4)], &data_0_3);

    assert_eq!(&mem[0x10000..(0x10000 + 4)], &data_1_0);

    assert_eq!(&mem[0x14000..(0x14000 + 4)], &data_1_1);

    assert_eq!(&mem[0x18000..(0x18000 + 4)], &data_1_2);

    assert_eq!(&mem[0x1C000..(0x1C000 + 4)], &data_1_3);

    // check the content of the saved file
    assert_eq!(&std::fs::read("good_bankset_0_0.o").unwrap(), &data_0_0);
    assert_eq!(&std::fs::read("good_bankset_0_1.o").unwrap(), &data_0_1);
    assert_eq!(&std::fs::read("good_bankset_0_2.o").unwrap(), &data_0_2);
    assert_eq!(&std::fs::read("good_bankset_0_3.o").unwrap(), &data_0_3);

    assert_eq!(&std::fs::read("good_bankset_1_0.o").unwrap(), &data_1_0);
    assert_eq!(&std::fs::read("good_bankset_1_1.o").unwrap(), &data_1_1);
    assert_eq!(&std::fs::read("good_bankset_1_2.o").unwrap(), &data_1_2);
    assert_eq!(&std::fs::read("good_bankset_1_3.o").unwrap(), &data_1_3);
}
