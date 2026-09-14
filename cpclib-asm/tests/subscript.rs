//! Postfix indexing/slicing: `target[i]` / `target[a..b]` / `target[x, y]`.
//! Baron's own "Subscripts" feature, backported after the docs were found
//! to already (incorrectly) claim `list[0]` worked - it didn't, this makes
//! it real.

#[test]
fn list_single_index() {
    let bin = cpclib_asm::assemble("org 0x4000\n l = [10,20,30]\n db l[1]\n").unwrap();
    assert_eq!(bin, vec![20]);
}

#[test]
fn list_index_out_of_range_errors() {
    assert!(cpclib_asm::assemble("org 0x4000\n l = [10,20,30]\n db l[3]\n").is_err());
}

#[test]
fn list_range_slice() {
    let bin = cpclib_asm::assemble("org 0x4000\n l = [10,20,30,40]\n db l[1..3]\n").unwrap();
    assert_eq!(bin, vec![20, 30]);
}

#[test]
fn list_literal_can_be_indexed_directly_without_a_named_variable() {
    let bin = cpclib_asm::assemble("org 0x4000\n db [10,20,30][2]\n").unwrap();
    assert_eq!(bin, vec![30]);
}

#[test]
fn chained_subscripts_on_a_nested_list() {
    let bin = cpclib_asm::assemble("org 0x4000\n l = [[1,2],[3,4]]\n db l[1][0]\n").unwrap();
    assert_eq!(bin, vec![3]);
}

#[test]
fn range_literal_can_be_indexed_directly_o1_no_materialization_needed() {
    let bin = cpclib_asm::assemble("org 0x4000\n db (0..1000000)[500000] & 0xff\n").unwrap();
    assert_eq!(bin, vec![(500000i32 & 0xff) as u8]);
}

#[test]
fn string_single_index_returns_a_char() {
    let bin = cpclib_asm::assemble("org 0x4000\n s = \"hello\"\n db s[1]\n").unwrap();
    assert_eq!(bin, vec![b'e']);
}

#[test]
fn string_range_slice_returns_a_substring() {
    let bin = cpclib_asm::assemble(
        "org 0x4000\n s = \"hello world\"\n assert s[0..5] == \"hello\"\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn matrix_x_y_two_index_form() {
    // A 2-wide, 3-tall matrix built from nested lists: rows [1,2],[3,4],[5,6].
    let bin = cpclib_asm::assemble(
        "org 0x4000\n m = matrix_new([[1,2],[3,4],[5,6]])\n assert m[0, 0] == 1\n assert \
         m[1, 2] == 6\n"
    );
    assert!(bin.is_ok(), "{bin:?}");
}

#[test]
fn matrix_single_index_is_an_error_two_indices_required() {
    assert!(
        cpclib_asm::assemble("org 0x4000\n m = matrix_new([[1,2],[3,4]])\n db m[0]\n").is_err()
    );
}

#[test]
fn two_indices_on_a_list_is_an_error() {
    assert!(cpclib_asm::assemble("org 0x4000\n l = [1,2,3]\n db l[0, 1]\n").is_err());
}

#[test]
fn list_of_indices_gathers_elements() {
    let bin =
        cpclib_asm::assemble("org 0x4000\n l = [10,20,30,40,50]\n db l[[0,2,4]]\n").unwrap();
    assert_eq!(bin, vec![10, 30, 50]);
}

#[test]
fn list_of_indices_out_of_range_errors() {
    assert!(cpclib_asm::assemble("org 0x4000\n l = [10,20,30]\n db l[[0,5]]\n").is_err());
}

#[test]
fn subscript_binds_tighter_than_arithmetic() {
    // a[0] + b[1] must be (a[0]) + (b[1]), not something weirder.
    let bin =
        cpclib_asm::assemble("org 0x4000\n a = [1,2]\n b = [10,20]\n db a[0] + b[1]\n").unwrap();
    assert_eq!(bin, vec![1 + 20]);
}
