extern "C" {
    fn APULTRA_crunch(
        data: *const libc::c_uchar,
        len: libc::c_int,
        dataout: *mut *mut libc::c_uchar,
        lenout: *mut libc::c_int
    ) -> libc::c_int;

    fn apultra_decompress(
        pInputData: *const libc::c_uchar,
        pOutBuffer: *const libc::c_uchar,
        nInputSize: libc::c_int,
        nMaxOutBufferSize: libc::c_int,
        nFlags: libc::c_uint
    ) -> libc::c_int;
}

/// Compress the given block using apultra method
pub fn compress(data: &[u8]) -> Vec<u8> {
    unsafe {
        let len = data.len() as libc::c_int;
        let data = data.as_ptr();

        let mut dataout: *mut libc::c_uchar = std::ptr::null_mut();
        let mut lenout: libc::c_int = 0;

        let _res = APULTRA_crunch(data, len, &mut dataout, &mut lenout);

        // copy the crunched C bytes in a rsut struct
        let crunched = {
            let mut crunched = Vec::new();
            crunched.reserve(lenout as usize);
            for idx in 0..(lenout as isize) {
                crunched.push(*dataout.offset(idx));
            }
            crunched
        };

        if lenout > 0 {
            libc::free(dataout as _);
        }

        crunched
    }
}
