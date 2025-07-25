#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
mod tests {
    use std::convert::{TryFrom, TryInto};

    use cpclib::disc::amsdos::*;
    use cpclib::disc::cfg::*;
    use cpclib::disc::edsk::ExtendedDsk;
    use cpclib_disc::disc::Disc;

    #[test]
    fn new_data() {
        let empty_expected = ExtendedDsk::open("./tests/dsk/empty.dsk").unwrap();
        let empty_obtained = ExtendedDsk::from(DiscConfig::single_head_data_format());
        assert_eq!(empty_expected.to_cfg(), empty_obtained.to_cfg());
    }

    #[test]
    fn get_onebasic_file() {
        let onefile = ExtendedDsk::open("./tests/dsk/onefile.dsk").unwrap();
        let manager = AmsdosManagerNonMut::new_from_disc(&onefile, 0);
        let file = manager
            .get_file(AmsdosFileName::try_from("test.bas").unwrap())
            .unwrap();
        assert!(dbg!(file.header().unwrap()).is_checksum_valid());
        assert!(file.is_basic());
        assert!(!file.is_ascii());
        assert_eq!(
            file.content().len() as u16,
            file.header().unwrap().file_length()
        );

        let file2 =
            AmsdosFile::basic_file_from_buffer(&"test.bas".try_into().unwrap(), file.content())
                .unwrap();

        assert_eq!(file.header(), file2.header());

        assert_eq!(file.content(), file2.content());

        let mut empty_obtained = ExtendedDsk::from(DiscConfig::single_head_data_format());
        let mut manager2 = AmsdosManagerMut::new_from_disc(&mut empty_obtained, 0);
        manager2
            .add_file(
                &file2,
                None,
                false,
                false,
                AmsdosAddBehavior::ReplaceAndEraseIfPresent
            )
            .unwrap();

        assert_eq!(manager.catalog(), manager2.catalog(),);

        let file3 = manager
            .get_file(AmsdosFileName::try_from("test.bas").unwrap())
            .unwrap();
        assert_eq!(file.header(), file3.header());

        assert_eq!(file.content(), file3.content());
    }

    #[test]
    fn list_catalog() {
        let dsk = cpclib::disc::edsk::ExtendedDsk::open("./tests/dsk/pirate.dsk").unwrap();
        let amsdos = cpclib::disc::amsdos::AmsdosManagerNonMut::new_from_disc(&dsk, 0);
        amsdos.print_catalog();
    }

    #[test]
    fn empty_catalog() {
        use cpclib::disc::amsdos::AmsdosManagerNonMut;
        use cpclib::disc::cfg::DiscConfig;

        let dsk: ExtendedDsk = DiscConfig::single_head_data_format().into();
        let manager = AmsdosManagerNonMut::new_from_disc(&dsk, 0);
        let catalog = manager.catalog();

        println!("{:?}", catalog);

        assert_eq!(catalog.used_entries().count(), 0);
    }

    #[test]
    fn test_hideur() {
        let content = [0x41, 0x42, 0x43, 0x0A];
        let result = [
            0, 116, 101, 115, 116, 32, 32, 32, 32, 98, 105, 110, 0, 0, 0, 0, 0, 0, 2, 0, 0, 16, 50,
            0, 4, 0, 52, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 11, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 65, 66, 67, 10
        ];
        let header = &result[0..128];

        let filename = AmsdosFileName::new_incorrect_case(0, "test", "bin").unwrap();
        let result_header =
            AmsdosHeader::compute_binary_header(&filename, 0x3210, 0x1234, &content);

        println!("{:?}", result_header);
        println!(
            "Obtained\t{:?}\nExpected\t{:?}\n",
            result_header.as_bytes().to_vec(),
            header.to_vec()
        );
        assert_eq!(result_header.as_bytes().to_vec(), header.to_vec());
    }

    #[test]
    fn test_amsdos_file() {
        let content = [0x41, 0x42, 0x43, 0x0A];
        let result = [
            0, 116, 101, 115, 116, 32, 32, 32, 32, 98, 105, 110, 0, 0, 0, 0, 0, 0, 2, 0, 0, 16, 50,
            0, 4, 0, 52, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 11, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 65, 66, 67, 10
        ];
        let _header = &result[0..128];

        let filename = AmsdosFileName::new_incorrect_case(0, "test", "bin").unwrap();
        let file =
            AmsdosFile::binary_file_from_buffer(&filename, 0x3210, 0x1234, &content).unwrap();

        let obtained_result = file.header_and_content().to_vec();
        assert_eq!(obtained_result.len(), result.len());
        assert_eq!(obtained_result, result.to_vec());
    }

    #[test]
    fn test_filename() {
        let fname1 = AmsdosFileName::new_correct_case(0, "test", "bin").unwrap();

        let fname2: AmsdosFileName = "TEST.BIN".try_into().unwrap();

        assert_eq!(fname1, fname2);
        assert_eq!(fname1.extension(), "BIN");
        assert_eq!(fname2.name(), "TEST");
        assert_eq!(fname2.user(), 0);

        let mut fname3 = fname2;
        fname3.set_extension("BAS");
        assert_eq!(fname3.extension(), "BAS");

        fname3.set_name("TOTOTO");
        assert_eq!(fname3.name(), "TOTOTO");

        assert_eq!(fname3.filename(), "TOTOTO.BAS");

        fname3.set_filename("HELLOWORLD");
        assert_eq!(fname3.name(), "HELLOWOR");
        assert_eq!(fname3.extension(), "");
    }

    #[test]
    fn test_filename_bytes() {
        let bytes = [
            0x00, 0x2D, 0x47, 0x57, 0x2D, 0x46, 0x52, 0x20, 0x20, 0x42, 0x41, 0x53
        ];
        let filename = AmsdosFileName::from_slice(&bytes);
        let result = filename.to_entry_format(false, false);

        println!("{:?}\n{:?}", &bytes, &result);
        assert_eq!(filename.user(), 0);
        assert_eq!(filename.name(), "-GW-FR");
        assert_eq!(filename.extension(), "BAS");
        assert_eq!(filename.filename(), "-GW-FR.BAS");
        assert_eq!(bytes, result);
    }

    #[test]
    fn test_entry() {
        let bytes = [
            0x00, 0x2D, 0x47, 0x57, 0x2D, 0x46, 0x52, 0x20, 0x20, 0x42, 0x41, 0x53, 0x00, 0x00,
            0x00, 0x06, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ];
        let entry = AmsdosEntry::from_buffer(0, &bytes);
        let file_results = entry.amsdos_filename().to_entry_format(false, false);
        let results = entry.as_bytes();

        println!(
            "Expected:\t{:?}\nObtained:\t{:?}",
            &bytes[..12],
            &file_results
        );

        assert_eq!(&bytes[..12], &file_results);

        println!("Expected:\t{:?}\nObtained:\t{:?}", &bytes, &results);

        assert_eq!(&bytes, &results);
    }

    #[test]
    fn add_file() {
        use cpclib::disc::cfg::DiscConfig;

        let mut filename = None;
        let mut file = None;
        let mut dsk: ExtendedDsk = DiscConfig::single_head_data_format().into();
        {
            let mut manager = AmsdosManagerMut::new_from_disc(&mut dsk, 0);
            let catalog = manager.catalog();

            assert_eq!(catalog.used_entries().count(), 0);

            assert_eq!(catalog.free_entries().count(), 64);

            filename.replace(AmsdosFileName::new_correct_case(0, "test", "bin").unwrap());
            assert_eq!(
                filename.as_ref().unwrap(),
                &AmsdosFileName::try_from("test.bin").unwrap()
            );

            file = Some(
                AmsdosFile::binary_file_from_buffer(
                    filename.as_ref().unwrap(),
                    0x3210,
                    0x1234,
                    &[0x41, 0x42, 0x43, 0x0A]
                )
                .unwrap()
            );
            manager
                .add_file(
                    file.as_ref().unwrap(),
                    None,
                    false,
                    false,
                    AmsdosAddBehavior::ReplaceAndEraseIfPresent
                )
                .expect("Unable to add file");

            assert_eq!(
                file.as_ref()
                    .unwrap()
                    .header()
                    .unwrap()
                    .amsdos_filename()
                    .unwrap()
                    .filename(),
                "TEST.BIN"
            );
            assert_eq!(
                &file
                    .as_ref()
                    .unwrap()
                    .header()
                    .unwrap()
                    .amsdos_filename()
                    .unwrap(),
                filename.as_ref().unwrap()
            );
            assert_eq!(
                file.as_ref().unwrap().header().unwrap().execution_address(),
                0x1234
            );
            assert_eq!(
                file.as_ref().unwrap().header().unwrap().loading_address(),
                0x3210
            );
        }

        let filename = filename.unwrap();
        let file = file.unwrap();

        let catalog_data = dsk.consecutive_sectors_read_bytes(0, 0, 0xC1, 4).unwrap();
        let entry_data = &catalog_data[..32];
        let entry = AmsdosEntry::from_slice(0, entry_data);
        println!("{:?}", entry_data);
        println!("{:?}", entry);
        assert_eq!(entry_data[0], entry.amsdos_filename().user());
        assert_eq!(entry.amsdos_filename().user(), 0);

        let manager = AmsdosManagerMut::new_from_disc(&mut dsk, 0);
        let catalog = manager.catalog();

        println!("{:?}", catalog);
        assert_eq!(catalog.used_entries().count(), 1);
        let entry = catalog.used_entries().next().unwrap();
        assert_eq!(
            entry.amsdos_filename(),
            &AmsdosFileName::try_from("test.bin").unwrap()
        );

        // TODO find a way to pass filename by reference
        let file2 = manager.get_file(filename);
        assert!(file2.is_some());
        let file2 = file2.unwrap();
        assert!(file2.header().unwrap().is_checksum_valid());
        assert_eq!(&file.header(), &file2.header());

        assert_eq!(&file.content().len(), &file2.content().len());

        assert_eq!(&file.content(), &file2.content());
    }
}
