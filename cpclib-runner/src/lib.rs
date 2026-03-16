#![feature(cfg_select)]

pub use enigo;
pub use xcap;

pub mod ace_config;
pub mod child_registry;
pub mod delegated;
pub mod embedded;
pub mod emucontrol;
pub mod runner;
pub use child_registry::kill_all_children;
pub use cpclib_common::event;
