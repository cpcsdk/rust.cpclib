use cpclib_common::event::DiscardObserver;
use cpclib_imgconverter::{fade_build_args, fade_handle_matches};

fn main() {
    let cmd = fade_build_args();
    let matches = cmd.get_matches();
    fade_handle_matches(&matches, &DiscardObserver).expect("Error in the generation");
}
