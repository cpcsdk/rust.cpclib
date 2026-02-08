use cpclib_common::winnow::Parser;

/// Generate a UTF-8 string from an Orgams binary file content.
pub fn binary_orgams_to_utf8(input: &[u8]) -> Result<String, String> {
	use crate::binary_decoder::{Input, parse_orgams_file};

	let mut input_wrapper = Input::new(input);
	let orgams_file = parse_orgams_file(false, &mut Option::<std::slice::Iter<String>>::None).parse_next(&mut input_wrapper)
		.map_err(|e| format!("Failed to parse Orgams binary file: {:?}", e))?;

	let output = orgams_file.to_string();
	Ok(output)
}


#[cfg(test)]
mod test{
    use crate::convert::binary_orgams_to_utf8;

const FILES_EXPECTED_TO_WORK: &[&str] = &[
	"MEMMAP.I",
	"SWAPI.I",
	"MONIQUE.I",
	"SYMBFLAG.I",
	"MACRO.I",
	"CONST.I",
	"bricbrac/BRICMAP.I",
	"orgext/EXTMAP.I",
	"orgext/ORGMAP.I",
	"orgass/ASSMAP.I",
	"monogams/MEMMAP.I",
	"monogams/MONOMAP.I",
	"monogams/CONST.I",
	"FARCALL.O",
	"EXCEPT.O",
	"bricbrac/AAP.O",
	"bricbrac/LISZT.O",
	"bricbrac/CONV-NRT.O",
	"bricbrac/CHECK.O",
	"bricbrac/HEAP.O",
	"bricbrac/STATUS.O",
	"bricbrac/TXTFIRM.O",
	"bricbrac/ASSET.O",
	"bricbrac/CONV.O",
	"bricbrac/SCR-LO.O",
	"bricbrac/CHUNK.O",
	"bricbrac/FIELD.O",
	"bricbrac/CUE.O",
	"bricbrac/IO.O",
	"bricbrac/STRING.O",
	"orgext/WRITE.O",
	"orgext/ORG.O",
	"orgext/FILENAME.O",
	"orgext/SWAP.O",
	"orgext/DISAROM.O",
	"orgext/DISA.O",
	"orgext/DISATST.O",
	"orgext/WRITETST.O",
	"orgext/DECEXP.O",
	"orgext/TOKEN.O",
	"orgext/DECEXPT.O",
	"orgext/ORGUI.O",
	"orgass/SYMB.O",
	"orgass/IMPORT.O",
	"orgass/CHUNKCC.O",
	"orgass/FIND.O",
	"orgass/VISU.O",
	"orgass/EVACMD.O",
	"orgass/CACHE.O",
	"orgass/COUNTNRT.O",
	"orgass/COCOPY.O",
	"orgass/ASSEVA.O",
	"orgass/COUNTROM.O",
	"orgass/COUNT.O",
	"orgass/PAGEFIRM.O",
	"orgass/IMPEVA.O",
	"orgass/ASSETO.O",
	"orgass/ASS.O",
	"orgass/COCOPROM.O",
	"monogams/TRTEST2.O",
	"monogams/MONHELP.O",
	"monogams/RSX.O",
	"monogams/TRTEST.O",
	"monogams/BT.O",
	"monogams/MON.O",
	"monogams/TRTEST1.O",
	"monogams/SEAHEX.O",
	"monogams/HISTRIOM.O",
	"orgams/HISTMOD.O",
	"orgams/BRK.O",
	"orgams/MULF.O",
	"orgams/UPD-SCR.O",
	"orgams/MIRROR.O",
	"orgams/OCCUPBAR.O",
	"orgams/OCCUPROM.O",
	"orgams/IMPEXP.O",
	"orgams/SYNHIGH.O",
	"orgams/OCCUPTST.O",
	"orgams/DECEXP.O",
	"orgams/EDNRT.O",
	"orgams/DETECT.O",
];


#[test]
fn check_string_generation_succeeds_without_verifying_content() {
	for base in FILES_EXPECTED_TO_WORK.iter() {

		let fname = std::path::Path::new("tests/orgams-main").join(base);
		println!("Generate the content of {}", fname.display());
		let content = fs_err::read(&fname).expect("Failed to read file");

		let unicode = binary_orgams_to_utf8(&content).expect("Failed to convert to UTF-8");
	}
}

}