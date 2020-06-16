///! Dummy manual c to rust adaptation of lz49 cruncher of Roudoudou
/// 

fn lz49_encode_extended_length(odata: &mut Vec<u8>,  mut length: usize)
{

	while length>=255 {
		odata.push(0xFF);
		length-=255;
	}
	/* if the last value is 255 we must encode 0 to end extended length */
	/*if (length==0) rasm_printf(ae,"bugfixed!\n");*/
	odata.push(length as u8);
}

fn lz49_encode_block(
	odata: &mut Vec<u8>, 
	data: &[u8], 
	mut literaloffset: usize, 
	literalcpt: usize,
	mut offset: usize, 
	maxlength: usize)
{
	let first_idx = odata.len()-1; // by construction is >0
	odata.push(0x00); // Will be overriden at the very last instruction


	let mut token=0;

	if/*offset<0 ||*/ offset>511 {
		panic!("internal offset error!\n");
	}
	
	if literalcpt<7 {
		token=literalcpt<<4; 
	} else {
		token=0x70;
		lz49_encode_extended_length(odata, literalcpt-7);
	}

	for i in 0..literalcpt {
		odata.push(data[literaloffset]);
		literaloffset +=1;
	}

	if maxlength<18 {
		if maxlength>2 {
			token |= maxlength-3;
		} else {
			/* endoffset has no length */
		}
	} else {
		token|=0xF;
		lz49_encode_extended_length(odata,maxlength-18);
	}

	if offset>255 {
		token|=0x80;
		offset-=256;
	}	
	if offset != 0 {
		odata.push((offset-1) as u8);
	}
	else {
		odata.push(255);
	}
	
	odata[first_idx] = token as u8;
}

/// Apply the lz49 crunching algorithm on the input data
/// This is just a dummy manual translation
pub fn lz49_encode_legacy(data: &[u8]) -> Vec<u8>
{

	assert!(data.len() > 0);

	let length = data.len();

	let mut current=1;
	let mut token;


	let mut literal=0;
	let mut literaloffset=1;

	let mut odata=Vec::new();
	odata.reserve(data.len() + data.len()/2 +10);
	
	/* first byte always literal */
	odata.push(data[0]);

	/* force short data encoding */
	if (length<5) {
		token=(length-1)<<4;
		odata.push(token as u8);
		for _i in 1..length {
			odata.push(data[current]);
			current +=1;
		}
		odata.push(0xFF);
		return odata;
	}

	while current<length {
		let mut maxlength=0;
		let mut startscan = {
			let mut startscan:i32 = current as i32-511;
			if startscan < 0 {
				startscan = 0;
			}
			startscan as usize
		};

		let mut matchlength;
		let mut curscan;
		let mut maxoffset=0;


		while startscan<current {
			matchlength = 0;
			curscan = current;
			{
				let mut i = startscan;
				while curscan<length {
					if data[i] == data[curscan] {
						curscan += 1;
						matchlength += 1;
					} else {
						curscan += 1;
						break;
					}
					i += 1;
				}
			}

			{
				if matchlength>=3 && matchlength>maxlength {
					maxoffset = startscan;
					maxlength = matchlength;
				}
			}
			startscan += 1;
		}
		if maxlength != 0 {
			lz49_encode_block(
				&mut odata, data, 
				literaloffset,
				literal,
				current-maxoffset,
				maxlength
			);
			current+=maxlength;
			literaloffset=current;
			literal=0;
		} else {
			literal+=1;
			current+=1;
		}
	}
	lz49_encode_block(
		&mut odata,
		data,
		literaloffset,
		literal,
		0,
		0
	);
	return odata;
}
