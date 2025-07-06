unsafe extern "C" {
    fn do_zx0_compress(
        input_data: *const libc::c_uchar,
        input_len: libc::c_int,
        retlen: *mut libc::c_int
    ) -> *const libc::c_uchar;

}

pub fn compress(data: &[u8]) -> Vec<u8> {
    unsafe {
        let len = data.len() as libc::c_int;
        let data = data.as_ptr();

        let mut lenout: libc::c_int = 0;

        let dataout = unsafe { do_zx0_compress(data, len, &mut lenout) };

        // copy the crunched C bytes in a rust struct
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
