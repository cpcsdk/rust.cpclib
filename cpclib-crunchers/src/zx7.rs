unsafe extern "C" {

    unsafe fn zx7_optimize(
        input_data: *const libc::c_uchar,
        input_size: libc::c_int
    ) -> *mut libc::c_void;

    unsafe fn ZX7_compress(
        optimal: *const libc::c_void,
        input_data: *const libc::c_uchar,
        input_size: libc::c_int,
        output_size: *mut libc::c_int
    ) -> *const libc::c_uchar;
}

pub fn compress(data: &[u8]) -> Vec<u8> {
    unsafe {
        let len = data.len() as libc::c_int;
        let data = data.as_ptr();
        let mut lenout: libc::c_int = 0;

        let optimal = zx7_optimize(data, len);

        let dataout = ZX7_compress(optimal, data, len, &mut lenout);

        // copy the crunched C bytes in a rust struct
        let crunched = {
            let mut crunched = Vec::with_capacity(lenout as usize);
            for idx in 0..(lenout as isize) {
                crunched.push(*dataout.offset(idx));
            }
            crunched
        };

        if lenout > 0 {
            libc::free(dataout as _);
        }
        libc::free(optimal); // XXX not done by roudoudou in rasm

        crunched
    }
}
