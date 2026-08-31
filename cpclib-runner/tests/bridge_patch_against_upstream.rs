//! The patch, applied to the real upstream files.
//!
//! The unit tests use a fixture shaped like `web/dist`; this one uses the
//! actual bytes from the pinned commit, so the anchors are checked against what
//! we will really be patching. Ignored by default because it needs the files on
//! disk - run it after a fetch, and whenever the pin moves:
//!
//! ```text
//! CPCLIB_1984JS_DIST=/path/to/web/dist \
//!   cargo test -p cpclib-runner --test bridge_patch_against_upstream -- --ignored
//! ```

use cpclib_common::camino::Utf8PathBuf;
use cpclib_runner::web::{BRIDGE_FILENAME, apply_bridge_patch};

#[test]
#[ignore = "needs an unpacked web/dist; set CPCLIB_1984JS_DIST"]
fn the_patch_applies_to_the_pinned_upstream() {
    let source = std::env::var("CPCLIB_1984JS_DIST")
        .ok()
        .map(Utf8PathBuf::from)
        .filter(|p| p.join("app.js").exists())
        .expect("set CPCLIB_1984JS_DIST to an unpacked 1984js web/dist");

    // Work on a copy: never touch the cached download itself.
    let tmp = camino_tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(source.as_std_path()).unwrap().flatten() {
        let path = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
        if path.is_file() {
            std::fs::copy(&path, tmp.path().join(path.file_name().unwrap())).unwrap();
        }
    }

    apply_bridge_patch(tmp.path()).expect("the patch must apply to the pinned commit");

    let index = std::fs::read_to_string(tmp.path().join("index.html")).unwrap();
    assert!(index.contains(BRIDGE_FILENAME));
    let app = std::fs::read_to_string(tmp.path().join("app.js")).unwrap();
    assert!(app.contains("__cpclib_attach"));

    // Applying twice changes nothing more.
    let once = app.clone();
    apply_bridge_patch(tmp.path()).unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("app.js")).unwrap(),
        once
    );
}
