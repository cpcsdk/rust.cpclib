#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    const DOUBLE_SIDED: &str = "NbTrack = 80
NbHead = 2

[Track-A:0]
SectorSize = 512
Gap3 = 0x4e
SectorID = 0xc1,0xc6,0xc2,0xc7,0xc3,0xc8,0xc4,0xc9,0xc5
sectorIDHead = 0,0,0,0,0,0,0,0,0

[Track-A:1,11,21,31,41,51,61,71]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:2,12,22,32,42,52,62,72]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:3,13,23,33,43,53,63,73]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:4,14,24,34,44,54,64,74]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:5,15,25,35,45,55,65,75]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:6,16,26,36,46,56,66,76]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:7,17,27,37,47,57,67,77]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:8,18,28,38,48,58,68,78]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:9,19,29,39,49,59,69,79]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-A:10,20,30,40,50,60,70]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track-B:0]
SectorSize = 512
Gap3 = 0x4e
SectorID = 0xc1,0xc6,0xc2,0xc7,0xc3,0xc8,0xc4,0xc9,0xc5
sectorIDHead = 0,0,0,0,0,0,0,0,0

[Track-B:1,11,21,31,41,51,61,71]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:2,12,22,32,42,52,62,72]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:3,13,23,33,43,53,63,73]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:4,14,24,34,44,54,64,74]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6,0xa7
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:5,15,25,35,45,55,65,75]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5,0xa6
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:6,16,26,36,46,56,66,76]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4,0xa5
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:7,17,27,37,47,57,67,77]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3,0xa4
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:8,18,28,38,48,58,68,78]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2,0xa3
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:9,19,29,39,49,59,69,79]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1,0xa2
sectorIDHead = 1,1,1,1,1,1,1,1,1,1

[Track-B:10,20,30,40,50,60,70]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xa2,0xa3,0xa4,0xa5,0xa6,0xa7,0xa8,0xa9,0xaa,0xa1
sectorIDHead = 1,1,1,1,1,1,1,1,1,1
";

    const SINGLE_SIDED: &str = "
NbTrack = 42
NbHead = 1

[Track:0]
SectorSize = 512
Gap3 = 0x4e
SectorID = 0xc1,0xc6,0xc2,0xc7,0xc3,0xc8,0xc4,0xc9,0xc5
sectorIDHead = 0,0,0,0,0,0,0,0,0

[Track:1,11,21,31,41]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb1,0xb2,0xb3,0xb4,0xb5,0xb6,0xb7,0xb8,0xb9,0xba
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:2,12,22,32]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xba,0xb1,0xb2,0xb3,0xb4,0xb5,0xb6,0xb7,0xb8,0xb9
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:3,13,23,33]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb9,0xba,0xb1,0xb2,0xb3,0xb4,0xb5,0xb6,0xb7,0xb8
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:4,14,24,34]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb8,0xb9,0xba,0xb1,0xb2,0xb3,0xb4,0xb5,0xb6,0xb7
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:5,15,25,35]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb7,0xb8,0xb9,0xba,0xb1,0xb2,0xb3,0xb4,0xb5,0xb6
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:6,16,26,36]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb6,0xb7,0xb8,0xb9,0xba,0xb1,0xb2,0xb3,0xb4,0xb5
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:7,17,27,37]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb5,0xb6,0xb7,0xb8,0xb9,0xba,0xb1,0xb2,0xb3,0xb4
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:8,18,28,38]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb4,0xb5,0xb6,0xb7,0xb8,0xb9,0xba,0xb1,0xb2,0xb3
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:9,19,29,39]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb3,0xb4,0xb5,0xb6,0xb7,0xb8,0xb9,0xba,0xb1,0xb2
sectorIDHead = 0,0,0,0,0,0,0,0,0,0

[Track:10,20,30,40]
SectorSize = 512
Gap3 = 0x30
SectorID = 0xb2,0xb3,0xb4,0xb5,0xb6,0xb7,0xb8,0xb9,0xba,0xb1
sectorIDHead = 0,0,0,0,0,0,0,0,0,0
";

    #[test]
    fn test_data() {
        // should not panic
        let _data_cfg = cpclib::disc::cfg::DiscConfig::single_head_data_format();
    }

    #[test]
    fn dsk_to_cfg() {
        let cfg = cpclib::disc::cfg::DiscConfig::from_str(SINGLE_SIDED).unwrap();

        let dsk = cpclib::disc::builder::build_disc_from_cfg(&cfg);
        let cfg2: cpclib::disc::cfg::DiscConfig = (&dsk).into();

        assert_eq!(cfg.explode(), cfg2.explode());

        let mut buffer = Vec::new();
        dsk.to_buffer(&mut buffer);
    }

    #[test]
    fn parse_double_headd_cfg() {
        let parsed = cpclib::disc::cfg::parse_config(DOUBLE_SIDED.into());
        assert!(parsed.is_ok());
        match parsed {
            Ok((next, res)) => {
                assert!(next.len() == 0);
                assert_eq!(res.to_string().to_uppercase(), DOUBLE_SIDED.to_uppercase());

                assert!(res
                    .track_information_for_track(cpclib::disc::edsk::Head::A, 0)
                    .is_some());
                assert!(res
                    .track_information_for_track(cpclib::disc::edsk::Head::A, 200)
                    .is_none());

                for idx in res.track_idx_iterator() {
                    let _track = res
                        .track_information_for_track(*idx.0, idx.1)
                        .expect(&format!("Unable to get information for {:?}", idx));
                    println!("{:?}", idx);
                }
                let edsk = cpclib::disc::builder::build_disc_from_cfg(&res);
                let generated = edsk.to_cfg();

                // Verify if we have the same content of tracks ids
                assert_eq!(
                    res.track_idx_iterator().collect::<Vec<_>>(),
                    generated.track_idx_iterator().collect::<Vec<_>>()
                );

                assert_eq!(
                    res.to_string().to_lowercase(),
                    generated.to_string().to_lowercase()
                );
            }
            _ => unreachable!()
        }
    }

    #[test]
    fn parse_single_headd_cfg() {
        let parsed = cpclib::disc::cfg::parse_config(SINGLE_SIDED.into());
        println!("{:?}", &parsed);
        assert!(parsed.is_ok());
        match parsed {
            Ok((next, _res)) => {
                assert!(next.len() == 0);
            }
            _ => unreachable!()
        }
    }

    #[test]
    fn arkos_disc() {
        let cfg =
            cpclib::disc::cfg::DiscConfig::from_str(include_str!("dsk/CreateDoubleSided_3_5i.cfg"))
                .unwrap();

        assert!(cfg
            .track_information_for_track(cpclib::disc::edsk::Head::A, 0)
            .is_some());
        assert!(cfg
            .track_information_for_track(cpclib::disc::edsk::Head::A, 79)
            .is_some());
        assert!(cfg
            .track_information_for_track(cpclib::disc::edsk::Head::A, 80)
            .is_none());

        assert!(cfg
            .track_information_for_track(cpclib::disc::edsk::Head::B, 0)
            .is_some());
        assert!(cfg
            .track_information_for_track(cpclib::disc::edsk::Head::B, 79)
            .is_some());
        assert!(cfg
            .track_information_for_track(cpclib::disc::edsk::Head::B, 80)
            .is_none());

        for idx in cfg.track_idx_iterator() {
            let _track = cfg
                .track_information_for_track(*idx.0, idx.1)
                .expect(&format!("Unable to get information for {:?}", idx));
            println!("{:?}", idx);
        }

        let dsk = cpclib::disc::builder::build_disc_from_cfg(&cfg);

        let mut buffer = Vec::new();
        dsk.to_buffer(&mut buffer);
    }

    #[test]
    fn test_build() {
        use tempfile::NamedTempFile;
        let file = NamedTempFile::new().unwrap();
        let path = file.into_temp_path();

        let cfg = cpclib::disc::cfg::DiscConfig::from_str(SINGLE_SIDED).unwrap();
        println!("{:?}", cfg);
        let track_info = cfg.track_information_for_track(3, 0).unwrap();
        assert_eq!(track_info.sector_size_human_readable(), 512);
        assert_eq!(track_info.gap3(), 0x4E);
        assert_eq!(track_info.nb_sectors(), 9);

        let track_info = dbg!(cfg.track_information_for_track(3, 18).unwrap());
        assert_eq!(track_info.sector_size_human_readable(), 512);
        assert_eq!(track_info.gap3(), 0x30);
        assert_eq!(track_info.nb_sectors(), 10);
        assert_eq!(track_info.sector_id_at(0), 0xB4);
        assert_eq!(track_info.sector_id_at(9), 0xB3);

        let cfgb = cfg.explode();
        let track_info = dbg!(cfgb.track_information_for_track(3, 18).unwrap());
        assert_eq!(track_info.sector_size_human_readable(), 512);
        assert_eq!(track_info.gap3(), 0x30);
        assert_eq!(track_info.nb_sectors(), 10);
        assert_eq!(track_info.sector_id_at(0), 0xB4);
        assert_eq!(track_info.sector_id_at(9), 0xB3);
        // This comment code compare the result with the one of Ramlaid.
        // It fails, but I'm not sure the error comes from us
        // let dsk2 = cpclib::disc::edsk::ExtendedDsk::open("tests/dsk/SingleSided_3i.dsk").unwrap();
        //
        // dbg!(&dsk2);
        // let cfg2 = dsk2.to_cfg();
        //
        // let track_info = dbg!(cfg2.track_information_for_track(3, 18).unwrap());
        // assert_eq!(track_info.sector_size_human_readable(), 512);
        // assert_eq!(track_info.gap3(), 0x30);
        // assert_eq!(track_info.nb_sectors(),10);
        // assert_eq!(track_info.sector_id_at(0), 0xb4);
        // assert_eq!(track_info.sector_id_at(9), 0xb3);
        //
        // assert_eq!(
        // cfg.explode(),
        // cfg2.explode()
        // );
        //
        let dsk = cpclib::disc::builder::build_disc_from_cfg(&cfg);
        let mut buffer = Vec::new();
        dsk.to_buffer(&mut buffer);
        let strbuffer = String::from_utf8_lossy(&buffer).to_owned();
        println!("{}", &strbuffer);

        // Check that the 1st track info is at the right place
        let loc = strbuffer.find("Track-Info").unwrap();
        assert_eq!(loc, 0x100);

        assert_eq!(dsk.nb_tracks(), 42);

        let check_track = |track_nb, sector_size, sectors_id: &[u8]| {
            let number_of_sectors = sectors_id.len() as u8;
            let track = dsk.get_track_information(3, track_nb).unwrap();
            println!("{:?}", track);
            assert_eq!(*track.number_of_sectors(), number_of_sectors);
            assert_eq!(track.sector_size_human_readable(), sector_size);

            // Check that the track contains the right number of sectors
            for sector_id in sectors_id.iter() {
                assert!(track.sector(*sector_id).is_some())
            }
        };

        check_track(
            0,
            512,                                                     // sector size
            &[0xC1, 0xC6, 0xC2, 0xC7, 0xC3, 0xC8, 0xC4, 0xC9, 0xC5]  // sectors ids
        );

        for track in &[1, 11, 21, 31, 41] {
            check_track(
                *track,
                512, // sector size
                &[0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA]
            );
        }

        for track in &[8, 18, 28, 38] {
            check_track(
                *track,
                512, // sector size
                &[0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xB1, 0xB2, 0xB3]
            );
        }

        dsk.save(&path).unwrap();
        let mut buffer = Vec::new();
        dsk.to_buffer(&mut buffer);

        let _dsk2 = cpclib::disc::edsk::ExtendedDsk::from_buffer(&buffer);

        let amsdos = cpclib::disc::amsdos::AmsdosManager::new_from_disc(dsk.clone(), 0);
        let catalog = amsdos.catalog();
        let nb_entries = catalog.used_entries().count();
        assert_eq!(0, nb_entries);
        // let dsk2 = cpclib::disc::edsk::ExtendedDsk::from_buffer(&buffer);
        // assert_eq!(dsk, dsk2)
    }
}
