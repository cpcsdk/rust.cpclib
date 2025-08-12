use cpclib_cpr::Cpr;

fn read_cartridge(fname: &str) {
    let cpr = Cpr::load(fname).expect("Error when reading CPR");
}

#[test]
fn read_burning_rubber() {
    read_cartridge("tests/Burning Rubber (1990)(Ocean Software).cpr")
}

#[test]
fn read_copter() {
    read_cartridge("tests/Copter 271 (1991)(Loriciels).cpr")
}
