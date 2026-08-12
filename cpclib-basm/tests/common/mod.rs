use std::sync::LazyLock;

pub static GLOBAL_TEST_LOCK: LazyLock<parking_lot::Mutex<()>> =
    LazyLock::new(parking_lot::Mutex::default);

pub fn manual_cleanup() {
    for fname in &[
        "BANK_C0.TXT",
        "BANK_C4.TXT",
        "BANK_C5.TXT",
        "BANK_C6.TXT",
        "BANK_C7.TXT",
        "good_bankset_0_0.o",
        "good_bankset_0_1.o",
        "good_bankset_0_2.o",
        "good_bankset_0_3.o",
        "good_bankset_1_0.o",
        "good_bankset_1_1.o",
        "good_bankset_1_2.o",
        "good_bankset_1_3.o",
        "good_save_txt.bin",
        "good_save_whole_inner.bin",
        "HELLO.BIN",
        "HELLO.TXT",
        "hello.bin",
        "hello.dsk",
        "hello.hfe",
        "hello1.bin",
        "hello2.bin",
        "hello3.bin",
        "lst.tmp",
        "TESTASCII.DSK"
    ] {
        let p = std::path::Path::new(fname);
        if p.exists() {
            fs_err::remove_file(p).unwrap()
        }
    }
}
