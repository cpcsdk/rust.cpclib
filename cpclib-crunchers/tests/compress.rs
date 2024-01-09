use cpclib_crunchers::{CompressMethod, shrinkler::ShrinklerConfiguration};

static DATA_TO_CRUNCH: &'static[u8] = "AAAAAAAAAAAAAAAAABNBNBNBNBAAAAAAAAACVCBCBCA".as_bytes();



fn crunch_any(method: CompressMethod) {
	let res = method.compress(DATA_TO_CRUNCH);
	dbg!(res.len(), DATA_TO_CRUNCH.len());
	assert!(res.len() < DATA_TO_CRUNCH.len());
}

#[test]
fn crunch_apultra() {
	crunch_any(CompressMethod::Apultra)
}

#[test]
fn crunch_exomizer() {
	crunch_any(CompressMethod::Exomizer)
}

#[test]
fn crunch_zx0() {
	crunch_any(CompressMethod::Zx0)
}

#[test]
fn crunch_shrinkler() {
	crunch_any(CompressMethod::Shrinkler(ShrinklerConfiguration{iterations:10, log: false}))
}

#[test]
fn crunch_lz4() {
	crunch_any(CompressMethod::Lz4)
}


#[test]
fn crunch_lz48() {
	crunch_any(CompressMethod::Lz48)
}


#[test]
fn crunch_lz49() {
	crunch_any(CompressMethod::Lz49)
}