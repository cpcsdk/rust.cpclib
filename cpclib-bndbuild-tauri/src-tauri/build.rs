/// Windows users may not be able to installa graphivz, so let's provide it
#[cfg(target_os = "windows")]
fn get_graphviz_resources() {
    use std::io::{Cursor, Read};
    use std::path::Path;

    use ureq;

    let url = "https://gitlab.com/api/v4/projects/4207231/packages/generic/graphviz-releases/12.2.1/windows_10_cmake_Release_Graphviz-12.2.1-win64.zip";
    let dst = Path::new("resources/Graphviz-12.2.1-win64");

    if !dst.exists() {
        let res = ureq::get(url)
            .header("Cache-Control", "max-age=1")
            .call()
            .expect("Unable to download graphviz/windows");
        let (_, mut body) = res.into_parts();
        let mut content = body.as_reader();

        let mut buffer = Vec::new();
        content.read_to_end(&mut buffer).unwrap();

        std::fs::create_dir_all(dst).expect("Unable to create resource dir");
        zip_extract::extract(Cursor::new(buffer), dst, true)
            .expect("Unable to extract graphivz/windows in resources folder");
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    get_graphviz_resources();

    tauri_build::build()
}
