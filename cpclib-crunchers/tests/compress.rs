use cpclib_crunchers::CompressMethod;
#[cfg(feature = "lzsa")]
use cpclib_crunchers::lzsa::LzsaMinMatch;
#[cfg(feature = "shrinkler")]
use cpclib_crunchers::shrinkler::ShrinklerConfiguration;

static DATA_TO_CRUNCH: &[u8] = "AAAAAAAAAAAAAAAAABNBNBNBNBAAAAAAAAACVCBCBCA".as_bytes();

fn crunch_any(method: CompressMethod) {
    let res = method.compress(DATA_TO_CRUNCH).unwrap();
    dbg!(res.len(), DATA_TO_CRUNCH.len());
    assert!(res.len() < DATA_TO_CRUNCH.len());
}

#[test]
#[cfg(feature = "apultra")]
fn crunch_apultra() {
    crunch_any(CompressMethod::Apultra)
}

#[test]
#[cfg(feature = "exomizer")]
fn crunch_exomizer() {
    crunch_any(CompressMethod::Exomizer)
}

#[test]
#[cfg(feature = "zx0")]
fn crunch_zx0() {
    crunch_any(CompressMethod::Zx0);
    crunch_any(CompressMethod::BackwardZx0)
}

#[test]
#[cfg(feature = "zx7")]
fn crunch_zx7() {
    crunch_any(CompressMethod::Zx7);
}

#[test]
#[cfg(feature = "pucrunch")]
fn crunch_pucrunch() {
    crunch_any(CompressMethod::Pucrunch);
}

#[test]
#[cfg(feature = "shrinkler")]
fn crunch_shrinkler() {
    crunch_any(CompressMethod::Shrinkler(ShrinklerConfiguration {
        iterations: 10,
        log: false
    }))
}

#[test]
#[cfg(feature = "lz4")]
fn crunch_lz4() {
    crunch_any(CompressMethod::Lz4)
}

#[test]
#[cfg(feature = "lz48")]
fn crunch_lz48() {
    crunch_any(CompressMethod::Lz48)
}

#[test]
#[cfg(feature = "lz49")]
fn crunch_lz49() {
    crunch_any(CompressMethod::Lz49)
}

#[test]
#[cfg(feature = "upkr")]
fn crunch_upkr() {
    crunch_any(CompressMethod::Upkr)
}

#[test]
#[cfg(feature = "lzsa")]
fn crunch_lzsa() {
    crunch_any(CompressMethod::Lzsa(
        cpclib_crunchers::lzsa::LzsaVersion::V1,
        None
    ));
    crunch_any(CompressMethod::Lzsa(
        cpclib_crunchers::lzsa::LzsaVersion::V2,
        None
    ));
    crunch_any(CompressMethod::Lzsa(
        cpclib_crunchers::lzsa::LzsaVersion::V1,
        Some(LzsaMinMatch::Val4)
    ));
    crunch_any(CompressMethod::Lzsa(
        cpclib_crunchers::lzsa::LzsaVersion::V2,
        Some(LzsaMinMatch::Val4)
    ));
}

#[test]
#[cfg(feature = "bzpack")]
fn crunch_bzpack() {
    crunch_any(CompressMethod::Lzm);
    crunch_any(CompressMethod::BackwardLzm);
    crunch_any(CompressMethod::Ef8);
    crunch_any(CompressMethod::BackwardEf8);
    crunch_any(CompressMethod::Bx0);
    crunch_any(CompressMethod::BackwardBx0);
    crunch_any(CompressMethod::Bx2);
    crunch_any(CompressMethod::BackwardBx2);
}
