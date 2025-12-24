unsafe extern "C" {
    fn LZ4_embedded_crunch(
        input_data: *const libc::c_uchar,
        input_len: libc::c_int,
        retlen: *mut libc::c_int
    ) -> *const libc::c_uchar;
}

pub fn compress(data: &[u8]) -> Vec<u8> {
    let len = data.len() as libc::c_int;
    let data = data.as_ptr();

    let mut lenout: libc::c_int = 0;

    let dataout = unsafe { LZ4_embedded_crunch(data, len, &mut lenout) };

    // copy the crunched C bytes in a rust struct
    let crunched = {
        let mut crunched = Vec::with_capacity(lenout as usize);
        for idx in 0..(lenout as isize) {
            crunched.push(unsafe { *dataout.offset(idx) });
        }
        crunched
    };

    if lenout > 0 {
        unsafe { libc::free(dataout as _) };
    }

    crunched
}
