#[cfg(test)]
mod tests {
    use cpclib::sna::*;

    #[test]
    pub fn sna_various() {
        let sna1 = Snapshot::load("tests/viewer.sna").unwrap();
        assert_eq!(3, sna1.nb_chunks());
        assert_eq!(sna1.memory_dump().len(), 64 * 1024);
        assert_eq!(sna1.memory_size_header(), 0);
        assert!(sna1.memory_block().is_empty());

        // Nothing should change by converting to a V3 snapshot (it is already one)
        let fixed = sna1.fix_version(SnapshotVersion::V3);
        assert_eq!(3, fixed.nb_chunks());
        assert_eq!(
            sna1.memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>(),
            fixed
                .memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>()
        );
        assert_eq!(fixed.memory_size_header(), 0);
        assert!(fixed.memory_block().is_empty());

        // Chunks must be removed in favor of the main memory
        let fixed = sna1.fix_version(SnapshotVersion::V2);
        assert_eq!(0, fixed.nb_chunks());
        assert_eq!(2, fixed.version_header());
        assert_eq!(64, fixed.memory_size_header()); // only one chunk was provided
        assert!(fixed.memory_block().is_64k());
        assert_eq!(
            sna1.memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>(),
            fixed
                .memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>()
        );

        // Check if convertd a v2 to v2 does not break stuff
        let v2 = fixed.fix_version(SnapshotVersion::V2);
        assert_eq!(0, v2.nb_chunks());
        assert_eq!(2, v2.version_header());
        assert_eq!(64, v2.memory_size_header()); // only one chunk was provided
        assert!(v2.memory_block().is_64k());
        assert_eq!(
            v2.memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>(),
            fixed
                .memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>()
        );

        // Save to reload (directly from V2)
        let f = tempfile::NamedTempFile::new().unwrap();
        let tmp_fname = f.path(); // TODO really use a tmp file
        v2.save(tmp_fname, SnapshotVersion::V2)
            .expect("Unable to save");

        let sna2 = Snapshot::load(tmp_fname).unwrap();
        assert_eq!(2, sna2.version_header());
        assert_eq!(sna1.memory_dump().len(), sna2.memory_dump().len());

        assert_eq!(
            sna1.memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>(),
            sna2.memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>()
        );

        // Should also work from V3
        let f = tempfile::NamedTempFile::new().unwrap();
        let tmp_fname = f.path();
        sna1.save(tmp_fname, SnapshotVersion::V2)
            .expect("Unable to save");

        let sna2 = Snapshot::load(tmp_fname).unwrap();
        assert_eq!(2, sna2.version_header());
        assert_eq!(sna1.memory_dump().len(), sna2.memory_dump().len());

        assert_eq!(
            sna1.memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>(),
            sna2.memory_dump()
                .iter()
                .map(|v| { (*v) as usize })
                .sum::<usize>()
        );
    }
}
