use crate::disc::cfg::DiscConfig;
use crate::disc::edsk::ExtendedDsk;


/// Generate an edsk from the given configuration
pub fn build_disc_from_cfg(cfg: &DiscConfig) -> ExtendedDsk {
	let mut edsk = ExtendedDsk::default();

	// Feed the disc info table
	edsk.disc_information_bloc.creator_name = "RUST CPC - BND".to_string();
	assert_eq!(
		edsk.disc_information_bloc.creator_name.len(),
		14);
	edsk.disc_information_bloc.number_of_sides = cfg.nb_sides;
	edsk.disc_information_bloc.number_of_tracks = cfg.nb_tracks;
	edsk.disc_information_bloc.track_size_table = cfg.track_idx_iterator()
					.map(|idx|{
						let info = cfg.track_information_for_track(
							*idx.0, idx.1)
						.unwrap_or_else(||panic!("Unable to acquire information for track {:?}", idx));
						let sectors_size = info.sector_size as usize * info.number_of_sectors();
						let header_size = 256;
						let encoded_size = ((sectors_size + header_size) /  256) as u8;
						encoded_size
					})
					.collect::<Vec<_>>();

	/// Create the empty tracks -- to be filled in the next loop
	for (side, track_idx) in cfg.track_idx_iterator() {
		let mut track = edsk.track_list.add_empty_track();
		track.track_number = track_idx;
		track.side_number = side.into();
		track.track_size = edsk.disc_information_bloc.track_size_table[track_idx as usize] as u16 * 256;
	}
	

	/// Update the tracks stuff
	for (&side, track_idx) in cfg.track_idx_iterator() {
		let track_info = edsk.get_track_information_mut(side, track_idx)
							.unwrap_or_else(||panic!("Unable to acquire track {} on side {:?} on the dsk", track_idx, side));
		let track_model = cfg.track_information_for_track(side, track_idx).unwrap();

		track_info.track_number = track_idx;
		track_info.side_number = track_model.side.into();
		track_info.sector_size = track_model.sector_size_dsk_format();
		track_info.number_of_sectors = track_model.sector_id.len() as _ ;
		track_info.gap3_length = track_model.gap3 as _; // TODO ensure a 8buts value is in the cfg
		track_info.filler_byte = 0xe5;	
		track_info.sector_information_list.fill_with(
			&track_model.sector_id,
			&track_model.sector_id_head,
			track_idx,
			track_model.sector_size_dsk_format(),
			track_info.filler_byte
		)
	}
	edsk
}


impl From<DiscConfig> for ExtendedDsk {
	fn from(config: DiscConfig) -> ExtendedDsk {
		build_disc_from_cfg(&config)
	}
}

impl From<&DiscConfig> for ExtendedDsk {
	fn from(config: &DiscConfig) -> ExtendedDsk {
		build_disc_from_cfg(config)
	}
}

pub fn single_side_data_dsk() -> ExtendedDsk {
	DiscConfig::single_side_data_format().into()
}

pub fn single_side_data42_dsk() -> ExtendedDsk {
	DiscConfig::single_side_data42_format().into()
}

