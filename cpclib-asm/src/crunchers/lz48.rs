/// ! Dummy manual c to rust adaptation of lz48 cruncher of Roudoudou

fn lz48_encode_extended_length(odata: &mut Vec<u8>, mut length: usize) {
    while length >= 255 {
        odata.push(0xFF);
        length -= 255;
    }
    // if the last value is 255 we must encode 0 to end extended length
    // if (length==0) rasm_printf(ae,"bugfixed!\n");
    odata.push(length as u8);
}

fn lz48_encode_block(
    odata: &mut Vec<u8>,
    data: &[u8],
    mut literaloffset: usize,
    literalcpt: usize,
    offset: usize,
    maxlength: usize
) {
    odata.push(0x00); // Will be overriden at the very last instruction
    let first_idx = odata.len() - 1; // by construction is >0

    if offset < 0 || offset > 255 {
        panic!("internal offset error!\n");
    }

    let mut token = if literalcpt < 15 {
        literalcpt << 4
    }
    else {
        lz48_encode_extended_length(odata, literalcpt - 15);
        0xF0
    };

    for _i in 0..literalcpt {
        odata.push(data[literaloffset]);
        literaloffset += 1;
    }

    if maxlength < 18 {
        if maxlength > 2 {
            token |= maxlength - 3;
        }
        else {
            // endoffset has no length
        }
    }
    else {
        token |= 0xF;
        lz48_encode_extended_length(odata, maxlength - 18);
    }

    odata.push(if offset == 0 { 255 } else { offset - 1 } as u8);

    odata[first_idx] = token as u8;
}

pub fn lz48_encode_legacy(data: &[u8]) -> Vec<u8> {
    assert!(data.len() > 0);

    let length = data.len();

    let token;

    let mut literal = 0;
    let mut literaloffset = 1;

    let mut odata = Vec::new();
    odata.reserve(data.len() + data.len() / 2 + 10);

    // first byte always literal
    let mut current = 0;
    odata.push(data[current]);
    current += 1;

    // force short data encoding
    if length < 5 {
        token = (length - 1) << 4;
        odata.push(token as u8);
        for _i in 1..length {
            odata.push(data[current]);
            current += 1;
        }
        odata.push(0xFF);
        return odata;
    }

    while current < length {
        let mut maxlength = 0;
        let mut startscan = if current < 255 { 0 } else { current - 255 };

        let mut maxoffset = 0;

        while startscan < current {
            let matchlength = {
                let mut matchlength = 0;
                let mut curscan = current;
                let mut i = startscan;
                while curscan < length && data[i] == data[curscan] {
                    matchlength += 1;
                    curscan += 1;
                    i += 1;
                }
                matchlength
            };

            if matchlength >= 3 && matchlength > maxlength {
                maxoffset = startscan;
                maxlength = matchlength;
            }
            startscan += 1;
        }

        if maxlength != 0 {
            lz48_encode_block(
                &mut odata,
                data,
                literaloffset,
                literal,
                current - maxoffset,
                maxlength
            );
            current += maxlength;
            literaloffset = current;
            literal = 0;
        }
        else {
            literal += 1;
            current += 1;
        }
    }
    lz48_encode_block(&mut odata, data, literaloffset, literal, 0, 0);

    return odata;
}
