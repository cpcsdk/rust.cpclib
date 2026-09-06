fn main() {
    let mut sna = cpclib_sna::Snapshot::new_6128().unwrap();
    sna.unwrap_memory_chunks().unwrap();

    let get16 = |addr: u32| -> u16 {
        let lo = sna.get_byte(addr) as u16;
        let hi = sna.get_byte(addr + 1) as u16;
        lo | (hi << 8)
    };

    println!("AE64 (end reserved area) = {:#06x}", get16(0xAE64));
    println!("AE62 (start reserved area) = {:#06x}", get16(0xAE62));
    println!("AE68 (var start) = {:#06x}", get16(0xAE68));
    println!("AE6A (arr start) = {:#06x}", get16(0xAE6A));
    println!("AE1D (cur line field ptr) = {:#06x}", get16(0xAE1D));
    println!("AE1B (cur stmt ptr) = {:#06x}", get16(0xAE1B));
    println!("ADB7 (chain A) = {:#06x}", get16(0xADB7));
    println!("bytes at 0x170: {:02x?}", (0x170u32..0x180).map(|a| sna.get_byte(a)).collect::<Vec<_>>());
}
