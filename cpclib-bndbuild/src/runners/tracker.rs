use std::fmt::Debug;

use cpclib_common::event::EventObserver;
use cpclib_runner::delegated::DelegateApplicationDescription;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tracker(cpclib_runner::runner::tracker::Tracker);

impl Tracker {
    delegate::delegate! {
        to self.0 {
            pub fn get_command(&self) -> &str;
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E>;
        }
    }

    pub fn new_at3_default() -> Self {
        Self(cpclib_runner::runner::tracker::Tracker::At3(
            Default::default()
        ))
    }

    pub fn new_chipnsfx_default() -> Self {
        Self(cpclib_runner::runner::tracker::Tracker::Chipnsfx(
            Default::default()
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SongConverter(cpclib_runner::runner::tracker::SongConverter);

macro_rules!  generate_song_handler{
    ($($name: ident, $fun: ident)*) => {
        $(
            pub fn $fun() -> Self {
                Self(cpclib_runner::runner::tracker::SongConverter::$name(
                    Default::default()
                ))
            }
        )*
    };
}
impl SongConverter {
    delegate::delegate! {
        to self.0 {
            pub fn get_command(&self) -> &str;
            pub fn configuration<E: EventObserver>(&self) -> DelegateApplicationDescription<E>;
        }
    }

    generate_song_handler! {
        SongToAkg, new_song_to_akg_default
        SongToAkm, new_song_to_akm_default
        SongToAky, new_song_to_aky_default
        SongToEvents, new_song_to_events_default
        SongToRaw, new_song_to_raw_default
        SongToSoundEffects, new_song_to_sound_effects_default
        SongToVgm, new_song_to_vgm_default
        SongToWav, new_song_to_wav_default
        SongToYm, new_song_to_ym_default
        Z80Profiler, new_z80profiler_default
    }
}
