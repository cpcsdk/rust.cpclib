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
}
