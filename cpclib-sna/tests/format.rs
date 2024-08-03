use std::fs::read;

use camino_tempfile::NamedUtf8TempFile;
use cpclib_sna::{Snapshot, SnapshotVersion};

#[test]
fn sna_loadv3() {
    let sna = Snapshot::new_6128().unwrap();
    assert_eq!(sna.version(), SnapshotVersion::V3);
}

#[test]
fn sna_loadv2() {
    println!("CWD: {}", std::env::current_dir().unwrap().display());
    let sna = Snapshot::load("tests/loop4000_v2.sna").unwrap();
    assert_eq!(sna.version(), SnapshotVersion::V2);

    let file = NamedUtf8TempFile::new().unwrap();
    let fname = file.path();
    sna.save(fname, SnapshotVersion::V2).unwrap();

    let src = include_bytes!("loop4000_v2.sna").to_vec();
    let tgt = read(fname).unwrap();

    similar_asserts::assert_eq!(src[..0x100], tgt[..0x100]);
    similar_asserts::assert_eq!(src[0x100..], tgt[0x100..]);
}

#[test]
fn sna_loadv3_savev2() {
    let sna = Snapshot::new_6128().unwrap();
    assert_eq!(sna.version(), SnapshotVersion::V3);

    let file = NamedUtf8TempFile::new().unwrap();
    let fname = file.path();

    sna.save(fname, SnapshotVersion::V2).unwrap();

    let sna2 = Snapshot::load(fname).unwrap();
    assert_eq!(sna2.version(), SnapshotVersion::V2);
}
