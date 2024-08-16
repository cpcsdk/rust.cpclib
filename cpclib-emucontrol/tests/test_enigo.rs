#!/usr/bin/env rust-script

//! ```cargo
//! [dependencies]
//! enigo = "0.2.1"
//! ```

use std::time::Duration;

use enigo::{Enigo, Key, Keyboard, Settings};

fn main() {
    std::thread::sleep(Duration::from_millis(1000));
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.text("#0").unwrap();
    enigo.key(Key::Return, enigo::Direction::Click).unwrap();
}
