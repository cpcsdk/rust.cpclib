use cpclib_image::palette;

#[test]
fn palette_from_strings() {
    let palette = palette![
        "black".to_owned(),
        "blue".to_owned(),
        "bright blue".to_owned(),
        "sky blue".to_owned(),
        "red".to_owned(),
        "bright red".to_owned(),
        "purple".to_owned(),
        "green".to_owned(),
        "bright green".to_owned(),
        "pastel green".to_owned()
    ];
}


#[test]
fn palette_from_str() {
    let palette = palette![
        "black",
        "blue",
        "bright blue",
        "sky blue",
        "red",
        "bright red",
        "purple",
        "green",
        "bright green",
        "pastel green"
    ];
}


#[test]
fn palette_from_numbers() {
    let palette = palette![
        0,
        1
    ];
}

#[test]
fn palette_from_mixes() {
    let palette = palette![
        0,
        "red"
    ];
}

#[test]
#[should_panic]
fn palette_fail_number() {
    let palette = palette![
        32
    ];
}

#[test]
#[should_panic]
fn palette_fail_str() {
    let palette = palette![
        "vert"
    ];
}