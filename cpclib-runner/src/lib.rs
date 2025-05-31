#![feature(cfg_select)]

pub use {enigo, xcap};

pub mod ace_config;
pub mod delegated;
pub mod embedded;
pub mod emucontrol;
pub mod runner;
pub use cpclib_common::event;
