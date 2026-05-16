use cpclib_common::clap::Parser;
use cpclib_common::event::DiscardObserver;
use cpclib_imgconverter::{FadeArgs, fade_process};

fn main() {
    let args = FadeArgs::parse();
    fade_process(&args, &DiscardObserver).expect("Error in the generation");
}
