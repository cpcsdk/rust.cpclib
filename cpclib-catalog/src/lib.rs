use cpclib_basic::BasicProgram;
use cpclib_catart::{char_command::CharCommandList, entry::{Catalog, CatalogType, PrintableEntry, PrintableEntryFileName, ScreenMode, UnifiedCatalog}};

pub fn display_catalog_using_catart(catalog_bytes: &[u8], catalog_type: CatalogType) -> Result<(), String> {
	let screen_output = catalog_screen_output(catalog_bytes, catalog_type)?;

	println!("{}", screen_output);
	
	Ok(())
}


pub fn catalog_screen_output(catalog_bytes: &[u8], catalog_type: CatalogType) -> Result<String, String> {
	let commands = catalog_to_catart_commands(catalog_bytes, catalog_type)?;
	Ok(commands.to_string())
}


pub fn catalog_extraction(catalog_bytes: &[u8], catalog_type: CatalogType) -> Result<Catalog, String> {

	assert_eq!(catalog_bytes.len(), 64*32);

    // Work directly with raw catalog bytes to avoid any filtering
    // Parse raw catalog bytes: each entry is 32 bytes
    // Byte 0 (St): Status/User number (0-15=user, 16-31=user P2DOS, 32=label, 33=timestamp, 0xE5=erased)
    // Bytes 1-8 (F0-F7): Filename (bit 7 = attributes)
    // Bytes 9-11 (E0-E2): Extension (bit 7 = attributes: E0=read-only, E1=system, E2=archived)
    // Byte 12 (Xl): Extent number low
    // Byte 13 (Bc): Byte count  
    // Byte 14 (Xh): Extent number high
    // Byte 15 (Rc): Record count
    // Bytes 16-31 (Al): Allocation blocks (16 bytes)
    
    // For CatArt disks, each entry is a separate display element, not extents of the same file
    // So we process each entry individually without grouping
    let mut printable_entries = Vec::new();
    
    // Process each 32-byte entry
    for chunk_idx in 0..64 {
        let offset = chunk_idx * 32;
        let entry_bytes = &catalog_bytes[offset..offset + 32];
        
        let status = entry_bytes[0];
        
        // Skip erased entries (0xE5), disc labels (32), and timestamps (33)
		// XXX should we keep them ?
        if status == 0xE5 {
            continue;
        }
        
        // Create PrintableEntryFileName directly from raw CP/M bytes
        // Bytes 1-8: filename (f1-f8), Bytes 9-11: extension (e1-e3)
        // IMPORTANT: Keep ALL bits as-is - bit 7 is part of CatArt encoding!
        let fname = PrintableEntryFileName {
            f1: entry_bytes[1],
            f2: entry_bytes[2],
            f3: entry_bytes[3],
            f4: entry_bytes[4],
            f5: entry_bytes[5],
            f6: entry_bytes[6],
            f7: entry_bytes[7],
            f8: entry_bytes[8],
            e1: entry_bytes[9],
            e2: entry_bytes[10],
            e3: entry_bytes[11]
        };
        
		let new_entry = PrintableEntry {
    		user: status,
    		fname,
    		pieces: [0; 4], // TODO use a real information
    		sectors: entry_bytes[16..32].try_into().unwrap()
		};

		// TODO handle files on several entries (we need to track this information)

        printable_entries.push(new_entry);
    }
    
    // Create catalog from entries
    let catalog = Catalog::try_from(printable_entries.as_slice())
        .map_err(|e| format!("Failed to create catalog: {}", e))?;

	Ok(catalog)
}



	
pub fn catalog_to_catart_commands(catalog_bytes: &[u8], catalog_type: CatalogType) -> Result<CharCommandList, String> {
	let catalog = catalog_extraction(catalog_bytes, catalog_type)?;

    // Convert to UnifiedCatalog
    let unified_catalog = UnifiedCatalog::from(catalog);
    
    // Get commands for display
    Ok(unified_catalog.commands_by_mode_and_order(ScreenMode::Mode1, catalog_type))
    
}



pub fn catalog_to_basic_listing(catalog_bytes: &[u8], catalog_type: CatalogType) -> Result<BasicProgram, String> {
	let catalog = catalog_extraction(catalog_bytes, catalog_type)?;

	// Get BASIC listing
	Ok(catalog.extract_basic_from_sequential_catart())
}
