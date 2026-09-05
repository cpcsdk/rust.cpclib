//! Build a `Box<str>` directly at an exact, caller-known size - no `Vec`,
//! no `String`, and (unlike `vec![0u8; len]`) no upfront zeroing either,
//! since every byte is about to be overwritten by `fill`.

use std::io::Cursor;
use std::mem::MaybeUninit;

/// Allocates `len` bytes directly as a `Box<[MaybeUninit<u8>]>`, hands
/// `fill` a byte-oriented cursor over that raw buffer to write through,
/// then casts the result straight to `Box<str>` once every byte has been
/// written.
///
/// `fill` must write to the cursor exactly `len` bytes of valid UTF-8 (in
/// practice: `&str` content copied verbatim, plus ASCII literals/digits) -
/// enforced by an `assert_eq!` on the cursor's final position before the
/// buffer is trusted to be fully initialized.
pub(crate) fn build_boxed_str(len: usize, fill: impl FnOnce(&mut Cursor<&mut [u8]>)) -> Box<str> {
    let mut boxed: Box<[MaybeUninit<u8>]> = Box::new_uninit_slice(len);

    // SAFETY: reinterpreting `&mut [MaybeUninit<u8>]` as `&mut [u8]` is
    // sound for *writing* even before every byte is initialized - `u8` has
    // no validity invariants beyond being any bit pattern, so nothing here
    // ever reads a byte before `fill` writes it.
    let ptr = boxed.as_mut_ptr().cast::<u8>();
    let byte_slice: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
    let mut cursor = Cursor::new(byte_slice);
    fill(&mut cursor);
    assert_eq!(
        cursor.position() as usize,
        len,
        "build_boxed_str: fill wrote fewer bytes than the buffer it was given - the \
         buffer would be left partially uninitialized, which is unsound to read back"
    );

    // SAFETY: every one of the `len` bytes was just written above, checked
    // by the assert.
    let bytes: Box<[u8]> = unsafe { boxed.assume_init() };
    // SAFETY: callers only ever write valid UTF-8 through the cursor (plain
    // &str content and ASCII literals/digits), and the assert above already
    // confirmed there is no leftover uninitialized tail.
    unsafe { std::str::from_boxed_utf8_unchecked(bytes) }
}
