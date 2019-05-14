#[cfg(test)]
mod tests {

    fn test_single_dsk(dsk: &cpclib::disc::edsk::ExtendedDsk) {
        let track = dsk
            .get_track_information(cpclib::disc::edsk::Head::HeadA, 0)
            .unwrap();
        assert_eq!(*track.number_of_sectors(), 9);

        for (sector_idx, sum) in &[
            (0xc1, 21413),
            (0xc6, 60263),
            (0xc2, 22014),
            (0xc7, 49447),
            (0xc3, 85780),
        ] {
            let sector = track.sector(*sector_idx).unwrap();
            let values = sector
                .values()
                .iter()
                .map(|&v| format!("{:x}", v))
                .collect::<Vec<_>>();
            println!("0x{:x} => {:?}", sector_idx, values);
            assert_eq!(values.len(), 512);
            assert_eq!(sector.data_sum(), *sum);
        }

        assert_eq!(track.data_sum(), 484121);
        assert_eq!(
            dsk.get_track_information(cpclib::disc::edsk::Head::HeadA, 41)
                .unwrap()
                .data_sum(),
            329484
        );

        // Check catalgo access

        assert!(dsk.sector(0, 0, 0xc1).is_some());
        assert!(dsk.sector(0, 0, 0xc2).is_some());
        assert!(dsk.sector(0, 0, 0xc3).is_some());
        assert!(dsk.sector(0, 0, 0xc4).is_some());

        assert!(dsk
            .sectors_bytes(
                0,    // track
                0xc1, // sector
                4,    //nb sector
                0.into()
            )
            .is_some());

        assert!(dsk
            .sectors_bytes(
                0,    // track
                0xc1, // sector
                4,    //nb sector
                1.into()
            )
            .is_none());

        assert!(dsk
            .sectors_bytes(
                0,    // track
                0xc1, // sector
                4,    //nb sector
                2.into()
            )
            .is_none());
    }

    #[test]
    fn open_singel_head_edsk() {
        let dsk = cpclib::disc::edsk::ExtendedDsk::open("./tests/dsk/pirate.dsk").unwrap();
        test_single_dsk(&dsk);

        let tmp_file = "/tmp/tmp.dsk";
        dsk.save(tmp_file);
        let dsk = cpclib::disc::edsk::ExtendedDsk::open(tmp_file).unwrap();
        test_single_dsk(&dsk);
    }

    fn test_double_head_bf_edsk(dsk: &cpclib::disc::edsk::ExtendedDsk) {
        assert!(dsk.is_double_head());
        assert_eq!(dsk.data_sum(cpclib::disc::edsk::Head::HeadA), 66709468);

        assert_eq!(dsk.data_sum(cpclib::disc::edsk::Head::HeadB), 54340792);
    }

    #[test]
    fn open_double_head_edsk() {
        let dsk = cpclib::disc::edsk::ExtendedDsk::open("./tests/dsk/bf2heads.dsk").unwrap();
        test_double_head_bf_edsk(&dsk);

        let tmp_file = "/tmp/tmp.dsk";
        dsk.save(tmp_file);
        let dsk = cpclib::disc::edsk::ExtendedDsk::open(tmp_file).unwrap();
        test_double_head_bf_edsk(&dsk);
    }

    #[test]
    fn save_edsk() {
        let tmp_file = "/tmp/tmp.dsk";
        let dsk1 = cpclib::disc::edsk::ExtendedDsk::open("tests/dsk/pirate.dsk").unwrap();
        dsk1.save(tmp_file);
        let _ds2 = cpclib::disc::edsk::ExtendedDsk::open(tmp_file).unwrap();
    }

    #[test]
    fn sector_size() {
        use cpclib::disc::edsk::convert_fdc_sector_size_to_real_sector_size;
        use cpclib::disc::edsk::convert_real_sector_size_to_fdc_sector_size;

        for real_human_size in [256, 512, 1024, 2048].iter() {
            let computed_fdc_size = convert_real_sector_size_to_fdc_sector_size(*real_human_size);
            let computed_human_size =
                convert_fdc_sector_size_to_real_sector_size(computed_fdc_size);

            assert_eq!(*real_human_size, computed_human_size);
        }
    }
}
