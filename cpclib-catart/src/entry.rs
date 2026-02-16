use std::borrow::Cow;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use bon::{Builder, builder};
use cpclib_basic::BasicProgram;
use cpclib_common::clap::Command;
use cpclib_common::itertools::Itertools;
use cpclib_common::smallvec::{SmallVec, smallvec};
use cpclib_image::image::Mode;
use owo_colors::OwoColorize;
use owo_colors::colors::css::ForestGreen;

use crate::basic_chars::{ACK, BS, CAN, DC1, DLE, EOT, ETB, NAK, SI, SO, SUB, US};
use crate::char_command::{CharCommand, CharCommandList};
use crate::{Locale, entry, interpret};

// Extract only the actual command bytes based on entry structure
//
// Special case: First entry with mode (f1 == 4 = EOT)
// f1: Mode command byte - PARSE THIS
// f2-f7: command bytes
// f8, e1: dot-hiding pair
// e2: command byte
//
// Regular case: Sequential basic (f1 > 4)
// f1: index for sorting - NOT DISPLAYED, NOT A COMMAND
// f2: ACK (0x06) - structural EnableVdu, skip
// f3-f7: 5 command bytes - PARSE THESE
// f8, e1: dot-hiding pair - one is structural, one is command byte
// e2: 1 command byte - PARSE THIS
// e3: NAK (0x15) - structural DisableVdu, skip
//
// let mut command_bytes = Vec::new();
//
// f1: Only parse if it's the first entry (Mode command)
// if self.f1 == 4 {
// First entry: f1 is the mode command byte (EOT = 0x04 = Mode)
// command_bytes.push(self.f1);
// }
// Otherwise f1 is just the sorting index, not included in commands
//
// f2: Skip if ACK (structural), otherwise parse
// if self.f2 != ACK {
// command_bytes.push(self.f2);
// }
//
// f3-f7: Always command bytes
// command_bytes.push(self.f3);
// command_bytes.push(self.f4);
// command_bytes.push(self.f5);
// command_bytes.push(self.f6);
// command_bytes.push(self.f7);
//
// Dot-hiding logic for f8/e1 pair:
// if self.f8 == ETB {
// AsModeParameter: f8=ETB (skip), e1=command byte
// command_bytes.push(self.e1);
// } else if self.e1 == BS {
// Erased: f8=command byte, e1=BS (skip)
// command_bytes.push(self.f8);
// } else if self.e1 == b'.' {
// Default: f8=command byte, e1='.' (skip)
// command_bytes.push(self.f8);
// } else {
// Fallback: include f8
// command_bytes.push(self.f8);
// }
//
// e2: Always a command byte
// command_bytes.push(self.e2);
//
// e3: Skip if NAK (structural)
// if self.e3 != NAK {
// command_bytes.push(self.e3);
// }
//
// Parse the filtered command bytes
// CharCommandList::from_bytes(&command_bytes)
//

/// The catalog may contain several entries for a given file
/// This directly represents the catalog maniplted by Amsdos
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    pub entries: [PrintableEntry; 64]
}

impl IntoIterator for Catalog {
    type IntoIter = std::array::IntoIter<PrintableEntry, 64>;
    type Item = PrintableEntry;

    fn into_iter(self) -> Self::IntoIter {
        std::array::IntoIter::new(self.entries)
    }
}

impl TryFrom<&[PrintableEntry]> for Catalog {
    type Error = String;

    fn try_from(value: &[PrintableEntry]) -> Result<Self, Self::Error> {
        if value.len() > 64 {
            return Err("Catalog must have 64 entries at maximum".to_string());
        }
        let mut entries = [PrintableEntry::empty(); 64];
        for (idx, entry) in value.into_iter().enumerate() {
            entries[idx] = *entry;
        }
        Ok(Catalog { entries })
    }
}

impl TryFrom<&[PrintableEntryFileName]> for Catalog {
    type Error = String;

    fn try_from(value: &[PrintableEntryFileName]) -> Result<Self, Self::Error> {
        let entries = value
            .iter()
            .map(|fname| PrintableEntry::from(*fname))
            .collect::<Vec<PrintableEntry>>();
        Catalog::try_from(entries.as_slice())
    }
}

impl Catalog {
    /// XXX does not take into account any ordering or mode.
    /// so it is quite buggy and can only serve for debbubing purposes
    /// TODO handle ordering and mode to be able to use this for catart generation
    pub fn to_basic_string(&self) -> String {
        self.entries()
            .map(|e| e.fname.commands().to_basic_string())
            .enumerate()
            .map(|(i, e)| format!("{} {}", i * 10, e))
            .join("\n")
    }

    // Here we want to obtain the basic program written by a human and injected in the catalaog.
    pub fn extract_basic_from_sequential_catart(&self, show_headers: bool) -> BasicProgram {
        let kind = CatalogType::Cat; // TODO handle this properly
        let mode = ScreenMode::Mode1; // TODO handle this properly
        let num_columns = 2; // kind.num_columns(mode);

        let unified = UnifiedCatalog::from(self.clone());
        let grid = EntriesGrid::from_entries(
            unified.visible_entries().map(Cow::Borrowed).collect(),
            num_columns,
            mode,
            kind
        );

        let mut enable = true;
        let mut entries = Vec::new();
        for (entry_nb, commands) in grid
            .entries_display_order()
            .map(|e| e.fname().commands()) // here we want the filename commands not the complete command
            .enumerate()
            .peekable()
        {
            let mut current_file = Vec::new();
            let mut current_string = Vec::new();

            let mut commands_iter = commands.into_iter().enumerate().peekable();
            while let Some((command_nb, c)) = commands_iter.next() {
                let c = c.normalize(); // TODO check if it is mandaotyr to do that

                // Handle the cat art encoding to show/hide stuff
                let is_catart_disabling =
                    matches!(c, CharCommand::DisableVdu) && commands_iter.peek().is_none();
                let is_catart_enabling =
                    matches!(c, CharCommand::EnableVdu) && command_nb == 1 && entry_nb != 0;
                let is_dot_handling = matches!(c, CharCommand::GraphicsInkMode(46));

                let is_nop = matches!(c, CharCommand::Nop);
                let is_enable = matches!(c, CharCommand::EnableVdu);
                let is_visible_char_handling =
                    matches!(c, CharCommand::Char(c) if c.is_ascii_graphic() && c!=b'"');

                if enable
                    && !is_catart_disabling
                    && !is_catart_enabling
                    && !is_nop
                    && !is_dot_handling
                {
                    if is_visible_char_handling {
                        current_string.push(c.first_byte());
                    }
                    else {
                        if !current_string.is_empty() {
                            current_file.push(CharCommand::String(current_string.clone()));
                            current_string.clear();
                        }
                        // CharCommands already have correct coordinates from parsing:
                        // - Window: already 1-based (from CharCommand::from_bytes)
                        // - Locate: already 0-based internal (from CharCommand::from_bytes)
                        // No transformation needed here.
                        current_file.push(c);
                    }
                }

                if is_catart_disabling {
                    enable = false;
                }
                else if is_catart_enabling {
                    enable = true;
                }
            }

            if !current_string.is_empty() {
                current_file.push(CharCommand::String(current_string.clone()));
            }
            entries.push(current_file);
        }

        let basic_str = entries
            .into_iter()
            .map(|cmds: Vec<CharCommand>| {
                cmds.into_iter()
                    .filter(|c| !matches!(c, CharCommand::GraphicsInkMode(b'.')))
            }) // remove dot handling commands as they are not needed in BASIC and just add noise
            .map(|cmds| cmds.into_iter().map(|c| c.bytes()).flatten()) // convert to the stream of bytes
            .map(|bytes| CharCommandList::from_bytes(&bytes.collect::<Vec<u8>>())) // convert it back to the stram of command (this allows to merge various stuff)
            .map(|cmds| cmds.to_basic_string())
            .enumerate()
            .map(|(i, s)| format!("{} {}", (i + 1) * 10, s))
            .join("\n");

        match BasicProgram::parse(&basic_str) {
            Ok(prog) => prog,
            Err(e) => {
                eprintln!("Failed to parse generated BASIC program:\n{}", basic_str);
                panic!("Parsing error: {}", e);
            }
        }
    }

    pub fn empty() -> Self {
        Catalog {
            entries: [PrintableEntry::empty(); 64]
        }
    }

    pub fn new(entries: [PrintableEntry; 64]) -> Self {
        Catalog { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| e.is_empty())
    }

    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| !e.is_empty()).count()
    }

    pub fn add(&mut self, entry: PrintableEntry) -> Result<(), String> {
        let available = self
            .entries
            .iter_mut()
            .find(|e| e.is_empty())
            .ok_or_else(|| "Catalog is full, cannot add more entries".to_string())?;

        *available = entry;
        Ok(())
    }

    pub fn entries(&self) -> impl Iterator<Item = &PrintableEntry> {
        self.entries.iter().filter(|e| !e.is_empty())
    }

    /// Convert the catalog to a byte slice (2048 bytes = 64 entries × 32 bytes)
    ///
    /// # Safety
    /// This method uses unsafe code to reinterpret the Catalog's memory as bytes.
    /// This is safe because Catalog is `#[repr(C)]` with a fixed layout.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let ptr = std::ptr::from_ref(self) as *const u8;
            std::slice::from_raw_parts(ptr, std::mem::size_of::<Catalog>())
        }
    }
}

/// The unified catalog contains only one entry per file and is allow to provide the size of a file (as its entries are merged)
/// DIR order is respected
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnifiedCatalog {
    pub entries: Vec<UnifiedPrintableEntry>
}

impl From<&[UnifiedPrintableEntry]> for UnifiedCatalog {
    fn from(value: &[UnifiedPrintableEntry]) -> Self {
        let entries = value.to_vec();
        UnifiedCatalog { entries }
    }
}

impl From<&[PrintableEntryFileName]> for UnifiedCatalog {
    fn from(value: &[PrintableEntryFileName]) -> Self {
        let entries = value
            .iter()
            .map(|fname| UnifiedPrintableEntry::from(*fname))
            .collect();
        UnifiedCatalog { entries }
    }
}

impl UnifiedCatalog {
    pub fn new(entries: [UnifiedPrintableEntry; 64]) -> Self {
        let entries_vec = entries.to_vec();
        UnifiedCatalog {
            entries: entries_vec
        }
    }

    pub fn empty() -> Self {
        UnifiedCatalog {
            entries: Vec::new()
        }
    }

    pub fn push(&mut self, entry: UnifiedPrintableEntry) -> Result<(), String> {
        // an entry is valid if no file has the same name and if there is space
        if self.entries.len() >= 64 {
            return Err("UnifiedCatalog is full, cannot add more entries".to_string());
        }
        if self.entries.iter().any(|e| e.fname == entry.fname) {
            return Err(
                "UnifiedCatalog already contains an entry with the same file name".to_string()
            );
        }
        self.entries.push(entry.into());
        Ok(())
    }

    pub fn size_kb(&self) -> u16 {
        self.entries.iter().map(|e| e.size_kb()).sum()
    }

    pub fn remaining_size_kb(&self) -> u16 {
        178 - self.size_kb()
    }
}

impl From<Catalog> for UnifiedCatalog {
    fn from(catalog: Catalog) -> Self {
        let mut entries: Vec<UnifiedPrintableEntry> = Vec::new();

        for raw_entry in catalog.into_iter() {
            if raw_entry.is_empty() {
                continue;
            }

            let entry_size = raw_entry.size_kb();

            if let Some(existing_entry) = entries.iter_mut().find(|e| e.fname == raw_entry.fname) {
                existing_entry.size_kb += entry_size;
            }
            else {
                entries.push(UnifiedPrintableEntry {
                    user: raw_entry.user,
                    fname: raw_entry.fname,
                    size_kb: entry_size
                });
            }
        }

        UnifiedCatalog { entries }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnifiedPrintableEntry {
    pub user: u8, // Currently ignored. TODO handle user
    pub fname: PrintableEntryFileName,
    pub size_kb: u16
}

impl From<Vec<PrintableEntry>> for UnifiedPrintableEntry {
    // this is valid if all entries have the same filename and user
    fn from(raw_entries: Vec<PrintableEntry>) -> Self {
        let mut iter = raw_entries.into_iter();
        let mut init: Self = iter.next().unwrap().into();
        for entry in iter {
            init.size_kb += entry.size_kb();
            // check the filename and user are the same
            assert_eq!(init.fname, entry.fname);
            assert_eq!(init.user, entry.user);
        }
        init
    }
}

impl From<PrintableEntry> for UnifiedPrintableEntry {
    fn from(raw: PrintableEntry) -> Self {
        UnifiedPrintableEntry {
            user: raw.user,
            fname: raw.fname,
            size_kb: raw.size_kb()
        }
    }
}

impl From<PrintableEntryFileName> for UnifiedPrintableEntry {
    fn from(raw: PrintableEntryFileName) -> Self {
        UnifiedPrintableEntry {
            user: 0,
            fname: raw,
            size_kb: 0
        }
    }
}

impl Deref for UnifiedPrintableEntry {
    type Target = PrintableEntryFileName;

    fn deref(&self) -> &Self::Target {
        &self.fname
    }
}

impl DerefMut for UnifiedPrintableEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fname
    }
}

impl UnifiedPrintableEntry {
    pub fn empty() -> Self {
        UnifiedPrintableEntry {
            user: 0,
            fname: PrintableEntryFileName::empty(),
            size_kb: 0
        }
    }

    pub fn fname(&self) -> &PrintableEntryFileName {
        &self.fname
    }

    // BUG does not handle file protection display
    pub fn all_generated_bytes(&self) -> [u8; 17] {
        let fname = self.fname.all_generated_bytes();
        let size = format!("{:>4}K", self.size_kb);
        let size = size.as_bytes();

        [
            fname[0], fname[1], fname[2], fname[3], fname[4], fname[5], fname[6], fname[7],
            fname[8], fname[9], fname[10], fname[11], size[0], size[1], size[2], size[3],
            size[4] /* not sure this is correct. Maybe some space are drawn wheareas they are supposed to be skip */
        ]
    }

    pub fn commands(&self) -> CharCommandList {
        let bytes = self.all_generated_bytes();
        let commands = bytes
            .iter()
            .map(|&b| CharCommand::Char(b))
            .collect::<Vec<CharCommand>>();
        CharCommandList::from(commands)
    }

    pub fn size_kb(&self) -> u16 {
        self.size_kb
    }
}

// https://cpc.sylvestre.org/technique/technique_catart1.html
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Builder)]
pub struct PrintableEntry {
    pub user: u8, // 0 to be printed
    pub fname: PrintableEntryFileName,
    pub pieces: [u8; 4],
    pub sectors: [u8; 16]
}

impl From<PrintableEntryFileName> for PrintableEntry {
    fn from(raw: PrintableEntryFileName) -> Self {
        let mut empty = Self::empty();
        if !raw.is_empty() {
            empty.user = 0;
            empty.sectors = [0; 16];
            empty.pieces = [0; 4];
            empty.fname = raw;
        }
        empty
    }
}

impl PrintableEntry {
    pub fn empty() -> Self {
        PrintableEntry {
            user: 0xE5,
            fname: PrintableEntryFileName::empty(),
            pieces: [0xE5; 4],
            sectors: [0xE5; 16]
        }
    }

    /// Create artificial entries from a filename and size.
    /// If the filename size is >16, multiple entries are created.
    pub fn artificial(fname: PrintableEntryFileName, size: u16) -> Vec<Self> {
        let mut entries = Vec::new();
        let mut remaining_size = size;
        let mut entry = PrintableEntry::from(fname);

        while remaining_size > 0 {
            let mut sectors = [0u8; 16];
            for i in 0..16 {
                if remaining_size > 0 {
                    sectors[i] = 1; // allocate one sector
                    remaining_size -= 1;
                }
                else {
                    sectors[i] = 0; // no more sectors needed
                }
            }
            entry.sectors = sectors;
            entries.push(entry);
            entry = PrintableEntry::from(fname); // create a new entry for next iteration
        }

        entries
    }

    /// An artificial entry is one that has no sectors allocated BUt a non empty filename.
    /// it is usually used for catart
    pub fn is_artificial(&self) -> bool {
        self.sectors.iter().all(|&s| s == 0) && !self.fname.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.fname.is_empty()
    }

    pub fn size_kb(&self) -> u16 {
        self.sectors.iter().filter(|&&s| s != 0).count() as u16
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PrintableEntryFileName {
    pub f1: u8,
    pub f2: u8,
    pub f3: u8,
    pub f4: u8,
    pub f5: u8,
    pub f6: u8,
    pub f7: u8,
    pub f8: u8,
    pub e1: u8,
    pub e2: u8,
    pub e3: u8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenMode {
    Mode0,
    Mode1,
    Mode2
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DotHiding {
    // Dot is an argument of mode change
    AsModeParameter,
    // Dot is reased after being printed
    Erased,
    // Dot is displayed as is
    KeptAsArgument
}

impl DotHiding {
    pub fn constraint_helper(&self) -> [usize; 2] {
        match self {
            DotHiding::AsModeParameter => [0, 1],
            DotHiding::Erased => [1, 0],
            DotHiding::KeptAsArgument => [1, 1]
        }
    }
}

/// A grid structure for organizing catalog entries by columns.
///
/// **Invariant**: The total number of entries across all columns never exceeds 64.
/// The grid display depends on screen mode and catalog type (cat or dir)
pub struct EntriesGrid<'cat> {
    drive: char,
    user: u8,
    mode: ScreenMode,
    order: CatalogType,
    columns: Vec<Vec<Cow<'cat, UnifiedPrintableEntry>>>
}

impl<'cat> EntriesGrid<'cat> {
    /// Create a new EntriesGrid from columns
    pub fn new(
        columns: Vec<Vec<Cow<'cat, UnifiedPrintableEntry>>>,
        mode: ScreenMode,
        order: CatalogType
    ) -> Self {
        let total_entries = columns.iter().map(|col| col.len()).sum::<usize>();
        assert!(
            total_entries <= 64,
            "EntriesGrid must never have more than 64 entries, got {}",
            total_entries
        );
        EntriesGrid {
            columns,
            mode,
            order,
            drive: 'A',
            user: 0
        }
    }

    /// Create a new EntriesGrid from entries and number of columns
    /// Entries are distributed into columns in row-major order (left to right, then next row)
    pub fn from_entries(
        entries: Vec<Cow<'cat, UnifiedPrintableEntry>>,
        num_columns: usize,
        mode: ScreenMode,
        order: CatalogType
    ) -> Self {
        assert!(
            entries.len() <= 64,
            "EntriesGrid must never have more than 64 entries, got {}",
            entries.len()
        );

        let entries = entries.into_iter().filter(|e| !e.is_system()); // remove not visible entries
        let mut columns: Vec<Vec<Cow<'cat, UnifiedPrintableEntry>>> = vec![vec![]; num_columns];
        for (i, entry) in entries.into_iter().enumerate() {
            columns[i % num_columns].push(entry);
        }

        EntriesGrid {
            columns,
            mode,
            order,
            drive: 'A',
            user: 0
        }
    }

    /// Get the number of columns
    pub fn num_columns(&self) -> usize {
        self.columns.len()
    }

    /// Get the total number of entries in the grid (always <= 64)
    pub fn len(&self) -> usize {
        self.columns.iter().map(|col| col.len()).sum()
    }

    /// Get the number of rows (maximum height of any column)
    pub fn max_num_rows(&self) -> usize {
        self.columns.iter().map(|col| col.len()).max().unwrap_or(0)
    }

    /// Get a reference to a specific column
    pub fn column(&self, index: usize) -> Option<&Vec<Cow<'cat, UnifiedPrintableEntry>>> {
        self.columns.get(index)
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&Cow<'cat, UnifiedPrintableEntry>> {
        self.columns.get(col).and_then(|column| column.get(row))
    }

    /// Iterate over rows, yielding an iterator over entries in each row
    pub fn rows(
        &self
    ) -> impl Iterator<Item = impl Iterator<Item = Option<&Cow<'cat, UnifiedPrintableEntry>>>> {
        let num_rows = self.max_num_rows();
        (0..num_rows).map(move |row| {
            (0..self.num_columns())
                .map(move |col| self.columns.get(col).and_then(|column| column.get(row)))
        })
    }

    /// Iterate over all entries in row-major order (left to right, then next row)
    pub fn entries_display_order(&self) -> impl Iterator<Item = &Cow<'cat, UnifiedPrintableEntry>> {
        self.rows().flatten().filter_map(|entry| entry)
    }

    /// Convert all borrowed entries to owned entries
    pub fn to_owned(&self) -> EntriesGrid<'static> {
        let owned_columns = self
            .columns
            .iter()
            .map(|column| {
                column
                    .iter()
                    .map(|entry| Cow::Owned((**entry).clone()))
                    .collect()
            })
            .collect();
        EntriesGrid::new(owned_columns, self.mode, self.order)
    }

    pub fn commands(&self) -> CharCommandList {
        self.commands_with_params(true)
    }

    pub fn commands_with_params(&self, show_headers: bool) -> CharCommandList {
        let mut available: u16 = 178;
        let mut commands = CharCommandList::new();

        if show_headers {
            // Inject the command that requires catalog display
            let cmd = match self.order {
                CatalogType::Cat => CharCommandList::from(b"cat"),
                CatalogType::Dir => CharCommandList::from(b"|dir")
            };
            commands.extend(cmd);
            commands.add_newlines(2);

            commands.extend(CharCommandList::from(
                format!("Drive {}: user  {}", self.drive, self.user).as_bytes()
            ));
            commands.add_newlines(2);
        }

        // Iterate over rows using the new grid structure
        for row_entries in self.rows() {
            for (col, entry) in row_entries.flatten().enumerate() {
                available = available.saturating_sub(entry.size_kb());
                commands.extend(entry.commands());

                let nb_spaces = match self.mode {
                    ScreenMode::Mode1 => 3,
                    _ => unimplemented!()
                };

                for _ in 0..nb_spaces {
                    commands.push(CharCommand::CursorRight);
                }
            }
        }

        if show_headers {
            commands.add_newlines(1);
            commands.extend(CharCommandList::from(
                format!("{:>3}K free", available).as_bytes()
            ));
            commands.add_newlines(2);
        }

        commands.into()
    }
}

/// Implementation of various builder patterns
/// https://cpc.sylvestre.org/technique/technique_catart2.html
impl PrintableEntryFileName {
    pub fn new<S: AsRef<str>>(fname: S) -> Self {
        // extract fname and extension
        let fname = fname.as_ref().to_ascii_uppercase();
        let (name, ext): (&str, &str) = if let Some(dot_pos) = fname.rfind('.') {
            let (name_part, ext_part) = fname.split_at(dot_pos);
            (name_part, &ext_part[1..])
        }
        else {
            (fname.as_str(), "")
        };

        assert!(name.len() <= 8, "Filename part too long: {}", name);
        assert!(ext.len() <= 3, "Extension part too long: {}", ext);

        let mut bytes = [b' '; 11];
        for (i, &b) in name.as_bytes().iter().take(8).enumerate() {
            bytes[i] = b;
        }
        for (i, &b) in ext.as_bytes().iter().take(3).enumerate() {
            bytes[8 + i] = b;
        }

        PrintableEntryFileName {
            f1: bytes[0],
            f2: bytes[1],
            f3: bytes[2],
            f4: bytes[3],
            f5: bytes[4],
            f6: bytes[5],
            f7: bytes[6],
            f8: bytes[7],
            e1: bytes[8],
            e2: bytes[9],
            e3: bytes[10]
        }
    }

    /// Helper method to handle dot hiding logic
    fn handle_dot_hiding(dot_hiding: DotHiding, display_char: u8) -> (u8, u8) {
        match dot_hiding {
            DotHiding::AsModeParameter => (ETB, display_char),
            DotHiding::Erased => (display_char, BS),
            DotHiding::KeptAsArgument => panic!("There is no hidden here")
        }
    }

    /// Ce type d'entrée du catalogue ne privilégiera aucune couleur d'affichage. C'est la méthode la plus standard, quoique jamais utilisée telle quelle. Elle permet d'afficher 3 caractères (représentés ici en vert vif) par entrée de catalogue. Sans doute la plus rentable en mode 0.
    ///
    /// 06 1F xx xx 0F xx 01 17 (point) 02 03 15
    pub fn unsorted_homogeneous(
        dot_hiding: DotHiding,
        x: u8,
        y: u8,
        pen: u8,
        char1: u8,
        char2: u8,
        char3: u8
    ) -> Self {
        let (f8, e1) = Self::handle_dot_hiding(dot_hiding, char2);
        PrintableEntryFileName {
            f1: ACK, // Enable VDU
            f2: US,  // Locate command
            f3: x,
            f4: y,
            f5: SI, // Pen command
            f6: pen,
            f7: char1, // char 1
            f8,
            e1,
            e2: char3,
            e3: NAK // Disable VDU
        }
    }

    pub fn unsorted_heterogeneous_first(
        dot_hiding: DotHiding,
        mode: u8,
        cmd1: u8,
        cmd2: u8,
        cmd3: u8,
        cmd4: u8,
        cmd5: u8,
        cmd6: u8,
        cmd7: u8
    ) -> Self {
        let (f8, e1) = Self::handle_dot_hiding(dot_hiding, cmd6);
        PrintableEntryFileName {
            f1: EOT, // Set mode; 4 is supposed to be printed first
            f2: mode,
            f3: cmd1,
            f4: cmd2,
            f5: cmd3,
            f6: cmd4,
            f7: cmd5,
            f8,
            e1,
            e2: cmd7,
            e3: NAK // Disable VDU
        }
    }

    /// Cette variante permet d'avoir davantage d'affichage sur une couleur, "identique à l'option homogène" sur une autre, et très faible sur les autres. Il faut considérer une couleur de base qui sera la norme de votre CATalogue et qui sera utilisée presque partout. On affichera de cette façon :
    ///
    /// 06 1F xx xx 01 02 03 17 (point) 04 05 15
    ///
    /// On pourra dans cette couleur afficher 5 caractères par entrée
    pub fn unsorted_heterogeneous_base_color(
        dot_hiding: DotHiding,
        x: u8,
        y: u8,
        char1: u8,
        char2: u8,
        char3: u8,
        char4: u8,
        char5: u8
    ) -> Self {
        let (f8, e1) = Self::handle_dot_hiding(dot_hiding, char4);
        PrintableEntryFileName {
            f1: ACK, // Enable VDU
            f2: US,  // Locate command
            f3: x,
            f4: y,
            f5: char1,
            f6: char2,
            f7: char3,
            f8,
            e1,
            e2: char5,
            e3: NAK // Disable VDU
        }
    }

    /// Une seconde couleur sera disponible par la technique citée plus loin du &18. Elle consiste à se mettre en mode transparent et définir un PAPER qui ne sera jamais utilisé, et qui servira de stockage pour un PEN rapidement accessible. En revanche, il faudra rétablir la couleur standard après utilisation :
    ///
    /// 06 1F xx xx 18 01 02 17 (point) 03 18 15
    ///
    /// On a accès à 3 caractères pour cette couleur.
    pub fn unsorted_heterogeneous_additional_color(
        dot_hiding: DotHiding,
        x: u8,
        y: u8,
        char1: u8,
        char2: u8,
        char3: u8
    ) -> Self {
        let (f8, e1) = Self::handle_dot_hiding(dot_hiding, char3);
        PrintableEntryFileName {
            f1: ACK, // Enable VDU
            f2: US,  // Locate command
            f3: x,
            f4: y,
            f5: CAN, // Exchange Pen and Paper
            f6: char1,
            f7: char2,
            f8,
            e1,
            e2: CAN, // Exchange Pen and Paper
            e3: NAK  // Disable VDU
        }
    }

    /// En revanche, toute autre couleur sera difficile d'accès, car il faudra déclarer manuellement la couleur, et restituer ensuite la couleur standard :
    ///
    /// 06 1F xx xx 0F xx 01 17 (point) 0F xx 15
    ///
    /// On n'a plus droit qu'à 1 caractère par entrée. Cette technique utilisée dans la compil Halloween est particulièrement intéressante en mode 1.
    pub fn unsorted_heterogeneous_extra_color(
        x: u8,
        y: u8,
        pen_extra: u8,
        pen_base: u8,
        char1: u8
    ) -> Self {
        PrintableEntryFileName {
            f1: ACK, // Enable VDU
            f2: US,  // Locate command
            f3: x,
            f4: y,
            f5: SI, // Pen command
            f6: pen_extra,
            f7: char1,
            f8: ETB, // graphic to ignore . eaten as a param
            e1: SI,  // Pen command
            e2: pen_base,
            e3: NAK // Disable VDU
        }
    }

    /// De façon bien concrète, regardons comment il est classique d'utiliser une entrée de CAT prévue pour fonctionner en mode séquentiel :
    ///
    /// AA 06 -- -- -- -- -- 17 (point) -- -- 15
    ///
    /// Le premier octet AA sera en fait l'index de l'entrée. Cet index ne sera pas affiché s'il est précédé d'une ligne identique à celle-ci. L'index sera suivi de &06 pour autoriser l'affichage des octets suivants. Le &17 (négociable selon les cas de figures avec d'autres bidouilles) sert à éviter l'affichage du point par le système. Il reste encore 2 octets affichés, et enfin l'octet &15 qui interdit l'affichage par le système des informations de taille de fichier et de ce précieux octet d'index qui commencera l'entrée suivante.
    pub fn sequential_basic(
        dot_hiding: DotHiding,
        index: u8,
        char1: u8,
        char2: u8,
        char3: u8,
        char4: u8,
        char5: u8,
        char6: u8,
        char7: u8
    ) -> Self {
        assert!(
            index > 4,
            "Index must be greater than 4 for sequential basic entries"
        );
        let (f8, e1) = Self::handle_dot_hiding(dot_hiding, char6);
        PrintableEntryFileName {
            f1: index, // Entry index (not displayed if preceded by identical line)
            f2: ACK,   // Enable display of following bytes
            f3: char1, // Displayable byte
            f4: char2, // Displayable byte
            f5: char3, // Displayable byte
            f6: char4, // Displayable byte
            f7: char5, // Displayable byte
            f8,
            e1,
            e2: char7, // Displayable byte
            e3: NAK    // Disable display of file size info and next entry's index
        }
    }

    /// La première entrée du CAT peut se présenter comme suit :
    ///
    /// 04 xx -- -- -- -- -- 17 (point) -- -- 15
    ///
    /// Si on est certain que &04 est le plus petit chiffre du CAT, afin qu'il soit au début (il est souvent utile de commencer par choisir un mode).
    pub fn sequential_first_with_mode(
        dot_hiding: DotHiding,
        mode: u8,
        char1: u8,
        char2: u8,
        char3: u8,
        char4: u8,
        char5: u8,
        char6: u8
    ) -> Self {
        let (f8, e1) = Self::handle_dot_hiding(dot_hiding, char5);
        PrintableEntryFileName {
            f1: b' ',  // Index
            f2: EOT,   // 4 for screen mode and 1st indexx
            f3: mode,  // sceren mode
            f4: char1, // Displayable byte
            f5: char2, // Displayable byte
            f6: char3, // Displayable byte
            f7: char4, // Displayable byte
            f8,
            e1,
            e2: char6, // Displayble byte
            e3: NAK    // Disable display of file size info and next entry's index
        }
    }

    /// Si on ne souhaite pas changer le mode, le &00 n'affichera rien, et l'entrée aura un octet libéré :
    ///
    /// 00 -- -- -- -- -- -- 17 (point) -- -- 15
    pub fn sequential_first_without_mode(
        dot_hiding: DotHiding,
        char1: u8,
        char2: u8,
        char3: u8,
        char4: u8,
        char5: u8,
        char6: u8,
        char7: u8,
        char8: u8
    ) -> Self {
        let (f8, e1) = Self::handle_dot_hiding(dot_hiding, char7);
        PrintableEntryFileName {
            f1: 0,     // Displays nothing, frees one byte
            f2: char1, // Displayable byte
            f3: char2, // Displayable byte
            f4: char3, // Displayable byte
            f5: char4, // Displayable byte
            f6: char5, // Displayable byte
            f7: char6, // Displayable byte
            f8,
            e1,
            e2: char8, // Displayble byte
            e3: NAK    // Disable display of file size info and next entry's index
        }
    }
}

impl PrintableEntryFileName {
    pub fn is_hidden(&self) -> bool {
        self.is_system() || self.is_empty()
    }

    pub fn is_system(&self) -> bool {
        self.e2 & 1 << 7 != 0
    }

    pub fn is_read_only(&self) -> bool {
        self.e1 & 1 << 7 != 0
    }
}

impl PrintableEntryFileName {
    pub fn empty() -> Self {
        PrintableEntryFileName {
            f1: 0xE5,
            f2: 0xE5,
            f3: 0xE5,
            f4: 0xE5,
            f5: 0xE5,
            f6: 0xE5,
            f7: 0xE5,
            f8: 0xE5,
            e1: 0xE5,
            e2: 0xE5,
            e3: 0xE5
        }
    }

    pub fn is_empty(&self) -> bool {
        self == &PrintableEntryFileName::empty()
    }

    pub fn filename(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(&self.f1 as *const u8, 8) }
    }

    pub fn extension(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(&self.e1 as *const u8, 3) }
    }

    /// List of generated bytes for this entry. This incude the separator dot.
    pub fn all_generated_bytes(&self) -> [u8; 12] {
        self.filename()
            .into_iter()
            .chain([&b'.'].into_iter())
            .chain(self.extension().into_iter())
            .cloned()
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }

    pub fn display_name(&self) -> String {
        if self.extension() == [b' ', b' ', b' '] {
            String::from_utf8_lossy(self.filename().as_ref())
                .trim_end_matches(|c: char| c == '\0' || c == ' ')
                .to_string()
        }
        else {
            String::from_utf8_lossy(self.all_generated_bytes().as_ref())
                .trim_end_matches(|c: char| c == '\0' || c == ' ')
                .to_string()
        }
    }

    /// Generate the CharCommandList for this entry
    /// but consider it can only work if the entry i
    /// TODO add a paramter to handle the commands per kind of entry
    pub fn commands(&self) -> CharCommandList {
        self.sequential_commands()
    }

    /// Generate the CharCommandList for this entry, considering it is a sequential entry.
    /// However it can fails when additional arguments are obtained from file size and so on.
    /// In that case wrong parameters are set to 0xFF to avoid panicing. So to not be really trusted
    pub fn sequential_commands(&self) -> CharCommandList {
        let bytes = self.all_generated_bytes();

        // hardcoded version for sequentail catalog entries
        if bytes[0] == 4 {
            let cmds = CharCommandList::from_bytes(&bytes);
            assert!(cmds[0].is_mode(), "First command must be mode setting");
            cmds
        }
        else {
            let order = CharCommand::Char(bytes[0]);
            let cmds = CharCommandList::from_bytes(&bytes[1..]);
            [order]
                .into_iter()
                .chain(cmds.into_iter())
                .collect::<Vec<CharCommand>>()
                .into()
        }
    }
}

pub enum CatArtMode {
    /// Each file requires a locate and we don't care of the display order
    Global,
    /// Each file requires a sorting key as we want a specific order
    Sequential
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogType {
    Cat,
    Dir
}

impl UnifiedCatalog {
    pub fn visible_entries(&self) -> impl Iterator<Item = &UnifiedPrintableEntry> {
        self.entries.iter().filter(|e| !e.is_hidden())
    }

    pub fn visible_sorted_entries(&self, order: CatalogType) -> Vec<&UnifiedPrintableEntry> {
        let mut entries: Vec<&UnifiedPrintableEntry> = self.visible_entries().collect();

        if let CatalogType::Cat = order {
            entries.sort_by_cached_key(|a| {
                let a = a.fname();
                let a_name = a
                    .filename()
                    .iter()
                    .chain(a.extension().iter())
                    .cloned()
                    .collect::<Vec<u8>>();
                a_name
            });
        }
        entries
    }

    pub fn visible_entries_by_mode_and_order(
        &self,
        mode: ScreenMode,
        order: CatalogType
    ) -> EntriesGrid {
        let entries = self.visible_sorted_entries(order);

        // TODO compute that with order or mode
        let num_columns = match mode {
            ScreenMode::Mode0 => 1,
            ScreenMode::Mode1 => 2,
            ScreenMode::Mode2 => 4
        };

        let mut columns: Vec<Vec<Cow<UnifiedPrintableEntry>>> = vec![vec![]; num_columns];

        match order {
            CatalogType::Cat => {
                // For CAT: fill columns horizontally (left to right, then next row)
                // but entries are already sorted alphabetically
                let max_height = (entries.len() + num_columns - 1) / num_columns; // ceil division
                let mut entries_iter = entries.into_iter();
                for col in 0..num_columns {
                    for _ in 0..max_height {
                        if let Some(entry) = entries_iter.next() {
                            assert!(!entry.is_system());
                            columns[col].push(Cow::Borrowed(entry));
                        }
                    }
                }
            },
            CatalogType::Dir => {
                // For DIR: fill columns horizontally (left to right, then next row)
                for (i, entry) in entries.into_iter().enumerate() {
                    columns[i % num_columns].push(Cow::Borrowed(entry));
                }
            }
        }

        EntriesGrid::new(columns, mode, order)
    }

    pub fn commands_by_mode_and_order(
        &self,
        mode: ScreenMode,
        order: CatalogType
    ) -> CharCommandList {
        self.commands_by_mode_and_order_with_params(mode, order, true)
    }

    pub fn commands_by_mode_and_order_with_params(
        &self,
        mode: ScreenMode,
        order: CatalogType,
        show_headers: bool
    ) -> CharCommandList {
        let grid = self.visible_entries_by_mode_and_order(mode, order);
        let commands = grid.commands_with_params(show_headers);
        let bytes = commands
            .iter()
            .flat_map(|cmd| cmd.bytes().into_iter())
            .collect::<Vec<_>>(); // ensure we merge commands
        CharCommandList::from_bytes(&bytes)
    }
}

impl From<EntriesGrid<'_>> for UnifiedCatalog {
    fn from(grid: EntriesGrid<'_>) -> Self {
        let mut entries = Vec::new();

        // Collect all entries from the grid in display order
        for entry in grid.entries_display_order() {
            entries.push((**entry).clone());
        }

        // Pad with empty entries if needed to reach 64
        while entries.len() < 64 {
            entries.push(UnifiedPrintableEntry::empty());
        }

        let entries_array: [UnifiedPrintableEntry; 64] = entries.try_into().unwrap();
        UnifiedCatalog::new(entries_array)
    }
}

/// XXX to be removed because it does not take into accound the  screen mode and cat/dir ordering
impl From<(UnifiedCatalog, ScreenMode, CatalogType)> for CharCommandList {
    fn from((catalog, screen, order): (UnifiedCatalog, ScreenMode, CatalogType)) -> Self {
        catalog.commands_by_mode_and_order(screen, order)
    }
}

impl Display for UnifiedCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let commands = self.commands_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);
        let mut interpreter = interpret::Interpreter::new_6128();
        interpreter
            .interpret(&commands, true)
            .map_err(|e| std::fmt::Error {})?;
        let content = interpreter.to_string();
        write!(f, "{}", content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryConstraintItem {
    Mode,   // Mode command is expected
    Locate, // Locate command is expected
    Pen,    // Pen command is expected
    Any(usize)
}

impl EntryConstraintItem {
    pub fn respect(&self, cmd: &CharCommand) -> bool {
        match self {
            EntryConstraintItem::Mode => matches!(cmd, CharCommand::Mode(_)),
            EntryConstraintItem::Locate => matches!(cmd, CharCommand::Locate(_, _)),
            EntryConstraintItem::Pen => matches!(cmd, CharCommand::Pen(_)),
            EntryConstraintItem::Any(size) => cmd.bytes().len() <= *size
        }
    }

    pub fn consume(self, cmd: &CharCommand) -> Option<EntryConstraintItem> {
        match self {
            EntryConstraintItem::Any(size) => {
                let cmd_size = cmd.bytes().len();
                if cmd_size < size {
                    Some(EntryConstraintItem::Any(size - cmd_size))
                }
                else {
                    None
                }
            },
            _ => {
                assert!(
                    self.respect(cmd),
                    "Command {:?} does not respect constraint {:?}",
                    cmd,
                    self
                );
                None
            }
        }
    }

    /// Real size of the instruction (+ parameters)
    pub fn real_size(&self) -> usize {
        match self {
            EntryConstraintItem::Mode => 2,
            EntryConstraintItem::Locate => 3,
            EntryConstraintItem::Pen => 2,
            EntryConstraintItem::Any(size) => *size
        }
    }

    /// Encoded size of the instruction.
    /// Can be smaller thatn the real size when the byte is encoded by construction
    /// (e.g. mode command in sequential first with mode)
    pub fn encoded_size(&self) -> usize {
        match self {
            EntryConstraintItem::Any(size) => *size,
            _ => self.real_size() - 1
        }
    }

    pub fn skip_first_byte(&self) -> bool {
        match self {
            EntryConstraintItem::Any(_) => false,
            _ => true
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryConstraint {
    slots: SmallVec<[EntryConstraintItem; 3]>
}

impl Deref for EntryConstraint {
    type Target = [EntryConstraintItem];

    fn deref(&self) -> &Self::Target {
        &self.slots
    }
}

impl EntryConstraint {
    pub fn real_size(&self) -> usize {
        self.iter().map(|item| item.real_size()).sum()
    }

    pub fn encoded_size(&self) -> usize {
        self.iter().map(|item| item.encoded_size()).sum()
    }

    pub fn iter(&self) -> impl Iterator<Item = &EntryConstraintItem> {
        self.slots.iter()
    }

    /// Consume the constraint according the command.
    /// Returns None if the constraint is fully consumed
    pub fn consume(mut self, cmd: &CharCommand) -> Option<Self> {
        if let Some(first) = self.slots.first_mut().cloned() {
            if let Some(remaining) = first.consume(cmd) {
                self.slots[0] = remaining;
                Some(self)
            }
            else {
                self.slots.remove(0);
                if self.slots.is_empty() {
                    None
                }
                else {
                    Some(self)
                }
            }
        }
        else {
            unreachable!("No more slots in constraint")
        }
    }

    /// Check if the command respect the constraint
    pub fn is_respected_by(&self, cmd: &CharCommand) -> bool {
        if let Some(first) = self.slots.first() {
            first.respect(cmd)
        }
        else {
            unreachable!("No more slots in constraint")
        }
    }

    pub fn consume_with_nops(mut self) -> usize {
        let mut nops = 0;
        loop {
            if let Some(remaining) = self.consume(&CharCommand::Nop) {
                self = remaining;
                nops += 1;
            }
            else {
                return nops + 1;
            }
        }
    }
}

/// Represents a familly of catart entries
pub enum EntryKind {
    // Enties are sequentially ordered and the first one must set the mode
    SequentialFirstWithMode(DotHiding),
    // Enties are sequentially ordered and the first does not set the mode
    SequentialFirstWithoutMode(DotHiding),
    // Entries are sequentially ordered. This one does not represents the first
    SequentialBasic(DotHiding),
    // Entries do not need to be sorted
    UnsortedHomogeneous(DotHiding),
    // Entries do not need to be sorted
    UnsortedHeterogeneousFirst(DotHiding),
    // Entries do not need to be sorted
    UnsortedHeterogeneousBaseColor(DotHiding),
    // Entries do not need to be sorted
    UnsortedHeterogeneousAdditionalColor(DotHiding)
}

impl EntryKind {
    pub fn requires_mode_as_first_command(&self) -> bool {
        match self {
            EntryKind::SequentialFirstWithMode(_) => true,
            _ => false
        }
    }

    /// Generate the bytes for the command.
    /// Crashes if the commands do not respect the constraint of the entry kind
    pub fn bytes_for_commands(&self, cmds: &[CharCommand]) -> SmallVec<[u8; 16]> {
        let constraint = self.constraint();
        let real_size: usize = constraint.real_size();

        let cmds_bytes: SmallVec<[SmallVec<[u8; 3]>; 4]> =
            cmds.iter().map(|c| c.bytes()).collect::<_>();
        let total_bytes = cmds_bytes.iter().map(|b| b.len()).sum::<usize>();
        assert!(
            real_size == total_bytes,
            "You must provide exactly {} bytes for this entry kind, got {}",
            real_size,
            total_bytes
        );

        let encoded_size = constraint.encoded_size();
        let mut bytes = SmallVec::with_capacity(real_size);

        let mut cmds_and_bytes = cmds.iter().zip(cmds_bytes.into_iter());
        for constraint_item in constraint.iter() {
            let mut remaining = constraint_item.encoded_size();
            while remaining > 0 {
                let (cmd, cmd_bytes) = cmds_and_bytes
                    .next()
                    .expect("Not enough commands provided for entry kind");
                assert!(
                    constraint_item.respect(cmd),
                    "Command {:?} does not respect constraint {:?}",
                    cmd,
                    constraint_item
                );

                let kept = if constraint_item.skip_first_byte() {
                    &cmd_bytes[1..]
                }
                else {
                    &cmd_bytes[..]
                };

                bytes.extend_from_slice(kept);
                remaining -= kept.len();
            }
        }
        assert!(
            bytes.len() == encoded_size,
            "Generated bytes length does not match expected size"
        );
        assert!(
            cmds_and_bytes.next().is_none(),
            "Too many commands provided for entry kind"
        );

        bytes
    }

    pub fn build_entry(&self, cmds: &[CharCommand], index: Option<u8>) -> PrintableEntryFileName {
        let args = self.bytes_for_commands(cmds); // by construction, we know this is the right amount of bytes
        match self {
            EntryKind::SequentialFirstWithMode(dot_hiding) => {
                PrintableEntryFileName::sequential_first_with_mode(
                    *dot_hiding,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6]
                )
            },
            EntryKind::SequentialFirstWithoutMode(dot_hiding) => {
                PrintableEntryFileName::sequential_first_without_mode(
                    *dot_hiding,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7]
                )
            },
            EntryKind::SequentialBasic(dot_hiding) => {
                PrintableEntryFileName::sequential_basic(
                    *dot_hiding,
                    index.unwrap(),
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6]
                )
            },
            EntryKind::UnsortedHomogeneous(dot_hiding) => {
                PrintableEntryFileName::unsorted_homogeneous(
                    *dot_hiding,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5]
                )
            },
            EntryKind::UnsortedHeterogeneousFirst(dot_hiding) => {
                PrintableEntryFileName::unsorted_heterogeneous_first(
                    *dot_hiding,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7]
                )
            },
            EntryKind::UnsortedHeterogeneousBaseColor(dot_hiding) => {
                PrintableEntryFileName::unsorted_heterogeneous_base_color(
                    *dot_hiding,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4],
                    args[5],
                    args[6]
                )
            },
            EntryKind::UnsortedHeterogeneousAdditionalColor(dot_hiding) => {
                PrintableEntryFileName::unsorted_heterogeneous_additional_color(
                    *dot_hiding,
                    args[0],
                    args[1],
                    args[2],
                    args[3],
                    args[4]
                )
            },
        }
    }

    fn constraint_helper(&self) -> [usize; 2] {
        match self {
            EntryKind::SequentialFirstWithMode(dot_hiding) => dot_hiding.constraint_helper(),
            EntryKind::SequentialFirstWithoutMode(dot_hiding) => dot_hiding.constraint_helper(),
            EntryKind::SequentialBasic(dot_hiding) => dot_hiding.constraint_helper(),
            EntryKind::UnsortedHomogeneous(dot_hiding) => dot_hiding.constraint_helper(),
            EntryKind::UnsortedHeterogeneousFirst(dot_hiding) => dot_hiding.constraint_helper(),
            EntryKind::UnsortedHeterogeneousBaseColor(dot_hiding) => dot_hiding.constraint_helper(),
            EntryKind::UnsortedHeterogeneousAdditionalColor(dot_hiding) => {
                dot_hiding.constraint_helper()
            },
        }
    }

    pub fn constraint(&self) -> EntryConstraint {
        let dot_help = self.constraint_helper();
        match self {
            EntryKind::SequentialFirstWithMode(_dot_hiding) => {
                EntryConstraint {
                    slots: smallvec![
                        EntryConstraintItem::Mode,
                        EntryConstraintItem::Any(4 + dot_help[0]),
                        EntryConstraintItem::Any(1 + dot_help[1])
                    ]
                }
            },
            EntryKind::SequentialFirstWithoutMode(_dot_hiding) => {
                EntryConstraint {
                    slots: smallvec![
                        EntryConstraintItem::Any(6 + dot_help[0]),
                        EntryConstraintItem::Any(1 + dot_help[1])
                    ]
                }
            },
            EntryKind::SequentialBasic(_dot_hiding) => {
                EntryConstraint {
                    slots: smallvec![
                        EntryConstraintItem::Any(5 + dot_help[0]),
                        EntryConstraintItem::Any(1 + dot_help[1])
                    ]
                }
            },
            EntryKind::UnsortedHomogeneous(_dot_hiding) => {
                EntryConstraint {
                    slots: smallvec![
                        EntryConstraintItem::Locate,
                        EntryConstraintItem::Pen,
                        EntryConstraintItem::Any(1 + dot_help[0]),
                        EntryConstraintItem::Any(1 + dot_help[1])
                    ]
                }
            },
            EntryKind::UnsortedHeterogeneousFirst(_dot_hiding) => {
                EntryConstraint {
                    slots: smallvec![
                        EntryConstraintItem::Mode,
                        EntryConstraintItem::Any(5 + dot_help[0]),
                        EntryConstraintItem::Any(1 + dot_help[1])
                    ]
                }
            },
            EntryKind::UnsortedHeterogeneousBaseColor(_dot_hiding) => {
                EntryConstraint {
                    slots: smallvec![
                        EntryConstraintItem::Locate,
                        EntryConstraintItem::Any(3 + dot_help[0]),
                        EntryConstraintItem::Any(1 + dot_help[1])
                    ]
                }
            },
            EntryKind::UnsortedHeterogeneousAdditionalColor(_dot_hiding) => {
                EntryConstraint {
                    slots: smallvec![
                        EntryConstraintItem::Locate,
                        EntryConstraintItem::Any(2 + dot_help[0]),
                        EntryConstraintItem::Any(0 + dot_help[1])
                    ]
                }
            },
        }
    }
}

/// Serial catalog builder. Only works with CAT
pub struct SerialCatalogBuilder {}

impl SerialCatalogBuilder {
    pub fn new() -> Self {
        SerialCatalogBuilder {}
    }

    /// Build a catalog from a list of char commands.
    /// This is a dummy approach that does not try to optimize the entries.
    /// So loss of space is guaranteed.
    pub fn build(&self, commands: &CharCommandList, start_mode: ScreenMode) -> UnifiedCatalog {
        if commands.is_empty() {
            return UnifiedCatalog::empty();
        }
        else {
            let entries = self.build_entries(commands, start_mode.clone());
            self.index_entries(entries, start_mode)
        }
    }

    fn build_entries(
        &self,
        commands: &CharCommandList,
        start_mode: ScreenMode
    ) -> Vec<PrintableEntryFileName> {
        assert!(
            commands.len() > 0,
            "Cannot build catalog from empty command list"
        );

        let first_is_mode = commands[0].is_mode();
        let mut entries = Vec::with_capacity(63);

        const FIRST_ENTRY_KIND_WITH_MODE: EntryKind =
            EntryKind::SequentialFirstWithMode(DotHiding::AsModeParameter);
        const FIRST_ENTRY_KIND_WITHOUT_MODE: EntryKind =
            EntryKind::SequentialFirstWithoutMode(DotHiding::AsModeParameter);
        const OTHER_ENTRY_KIND: EntryKind = EntryKind::SequentialBasic(DotHiding::AsModeParameter);

        let first_entry = if first_is_mode {
            FIRST_ENTRY_KIND_WITH_MODE
        }
        else {
            FIRST_ENTRY_KIND_WITHOUT_MODE
        };
        let other_entry_kinds = OTHER_ENTRY_KIND;

        let mut current_constraint = first_entry.constraint();
        let mut commands_buffer = SmallVec::<[CharCommand; 3]>::with_capacity(3);

        // build the entries one by one. However their numbering is wrong for now
        let mut cmd_idx = 0;
        while cmd_idx < commands.len() {
            let command = &commands[cmd_idx];

            let constraint_is_respected = current_constraint.is_respected_by(command);
            let (force_nops, create_entry) = if !constraint_is_respected {
                (true, false)
            }
            else {
                cmd_idx += 1; // as we accept the command we need to move to the next one
                // Command respects the current constraint and can be  added
                commands_buffer.push(command.clone()); // right size and kind
                let force_nops = cmd_idx == commands.len(); // we force nops if we are at the last command

                let copied_constraint = current_constraint.clone();
                let create_entry =
                    if let Some(remaining_constraint) = copied_constraint.consume(command) {
                        // we still have remaining constraint to fill
                        current_constraint = remaining_constraint;
                        false
                    }
                    else {
                        true
                    };

                (force_nops, create_entry)
            };

            if create_entry || force_nops {
                // if we have still constraints while we are at the end of commands, we need to fill with no-op commands
                if force_nops && !create_entry {
                    let copied_constraint = current_constraint.clone();
                    let nb_nops = copied_constraint.consume_with_nops();
                    for _ in 0..nb_nops {
                        commands_buffer.push(CharCommand::Nop);
                    }
                }

                //  set the constraint othe next entry
                current_constraint = other_entry_kinds.constraint();

                // Constraint fully consumed, we can create entry
                let entry = if entries.is_empty() {
                    first_entry.build_entry(&commands_buffer, None)
                }
                else {
                    other_entry_kinds
                        .build_entry(&commands_buffer, Some((entries.len() as u8 + b' ') as u8))
                };

                if entries.len() >= 64 {
                    panic!("Catalog full, cannot add more entries");
                }
                entries.push(entry);

                commands_buffer.clear();
                // Set next constraint
            }
        }

        entries
    }

    pub fn index_entries(
        &self,
        entries: Vec<PrintableEntryFileName>,
        start_mode: ScreenMode
    ) -> UnifiedCatalog {
        // update the numbers of the entries because they were created without correct numbering.
        // Their current number is the number in the cataog + 4 (as per spec) but now we it to become the number in the display grid row by row
        // Remember to display in the grid the entries or order by their number in a column, but they are displayed row by row
        let mut cat = UnifiedCatalog::from(entries.as_slice());
        assert_eq!(
            cat.visible_entries().count(),
            entries.len(),
            "Number of entries in catalog does not match provided entries"
        );
        let grid = cat.visible_entries_by_mode_and_order(start_mode, CatalogType::Cat);

        let ordered_idx = grid
            .entries_display_order()
            .enumerate()
            .map(|(new_idx, entry)| entry.f1)
            .collect::<Vec<u8>>();
        for (entry, new_number) in cat.entries.iter_mut().zip(ordered_idx.into_iter()) {
            if !entry.is_empty() {
                entry.f1 = new_number;
            }
        }

        cat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::Mode;

    fn create_test_entries() -> [PrintableEntry; 64] {
        let mut entries = Vec::new();

        // Create some test entries with different names for sorting
        entries.push(PrintableEntry::from(PrintableEntryFileName {
            f1: b'z',
            f2: b'e',
            f3: b'b',
            f4: b'r',
            f5: b'a',
            f6: b' ',
            f7: b' ',
            f8: b' ',
            e1: b' ',
            e2: b' ',
            e3: b' '
        }));
        entries.push(PrintableEntry::from(PrintableEntryFileName {
            f1: b'a',
            f2: b'p',
            f3: b'p',
            f4: b'l',
            f5: b'e',
            f6: b' ',
            f7: b' ',
            f8: b' ',
            e1: b' ',
            e2: b' ',
            e3: b' '
        }));
        entries.push(PrintableEntry::from(PrintableEntryFileName {
            f1: b'b',
            f2: b'a',
            f3: b'n',
            f4: b'a',
            f5: b'n',
            f6: b'a',
            f7: b' ',
            f8: b' ',
            e1: b' ',
            e2: b' ',
            e3: b' '
        }));
        entries.push(PrintableEntry::from(PrintableEntryFileName {
            f1: b'c',
            f2: b'h',
            f3: b'e',
            f4: b'r',
            f5: b' ',
            f6: b' ',
            f7: b' ',
            f8: b' ',
            e1: b' ',
            e2: b' ',
            e3: b' '
        }));
        entries.push(PrintableEntry::from(PrintableEntryFileName {
            f1: b'd',
            f2: b'o',
            f3: b'g',
            f4: b' ',
            f5: b' ',
            f6: b' ',
            f7: b' ',
            f8: b' ',
            e1: b' ',
            e2: b' ',
            e3: b' '
        }));

        // Fill the rest with dummy entries
        for i in 5..64 {
            let name = format!("file{:02}", i);
            let bytes = name.as_bytes();
            entries.push(PrintableEntry::from(PrintableEntryFileName {
                f1: bytes[0],
                f2: bytes[1],
                f3: bytes[2],
                f4: bytes[3],
                f5: bytes[4],
                f6: bytes[5],
                f7: b' ',
                f8: b' ',
                e1: b' ',
                e2: b' ',
                e3: b' '
            }));
        }

        entries.try_into().expect("Failed to create test entries")
    }

    #[test]
    fn test_entries_by_mode_and_order_mode0() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog);
        let result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode0, CatalogType::Cat);

        // Mode0 should have 1 column
        assert_eq!(result.num_columns(), 1);
        assert_eq!(result.column(0).unwrap().len(), 64);
    }

    #[test]
    fn test_entries_by_mode_and_order_mode1() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog);
        let result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);

        // Mode1 should have 2 columns
        assert_eq!(result.num_columns(), 2);
        // Check that entries are distributed (may not be exactly equal due to odd total)
        assert!(result.column(0).unwrap().len() >= 31 && result.column(0).unwrap().len() <= 33);
        assert!(result.column(1).unwrap().len() >= 31 && result.column(1).unwrap().len() <= 33);
        assert_eq!(
            result.column(0).unwrap().len() + result.column(1).unwrap().len(),
            64
        );
    }

    #[test]
    fn test_entries_by_mode_and_order_mode2() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog);
        let result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Cat);

        // Mode2 should have 4 columns
        assert_eq!(result.num_columns(), 4);
        // Check total entries
        let total: usize = (0..result.num_columns())
            .map(|i| result.column(i).unwrap().len())
            .sum();
        assert_eq!(total, 64);
    }

    #[test]
    fn test_entries_by_mode_and_order_cat_sorting() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog);
        let result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode0, CatalogType::Cat);

        // Should be sorted alphabetically (apple, banana, cher, dog, file05, ...)
        let first_five: Vec<String> = result
            .column(0)
            .unwrap()
            .iter()
            .take(5)
            .map(|entry| entry.display_name())
            .collect();

        assert_eq!(first_five, vec!["apple", "banana", "cher", "dog", "file05"]);
    }

    #[test]
    fn test_entries_by_mode_and_order_dir_no_sorting() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog);
        let result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode0, CatalogType::Dir);

        // Should maintain original order (zebra, apple, banana, cher, dog, ...)
        let first_five: Vec<String> = result
            .column(0)
            .unwrap()
            .iter()
            .take(5)
            .map(|entry| entry.display_name())
            .collect();

        assert_eq!(first_five, vec!["zebra", "apple", "banana", "cher", "dog"]);
    }

    #[test]
    fn test_entries_by_mode_and_order_catart_examples() {
        // Create entries matching the CatArt documentation examples
        let mut entries = Vec::new();

        // Add the specific entries from the examples in the order they appear in DIR
        // Note: NotIndexed entries only store 6 characters for filenames
        // Using simplified names that demonstrate the sorting and distribution
        let example_names = vec![
            "FILEG", "FILEB", "FILEJ", "FILED", "FILEK", "FILEM", "FILEC", "FILEL", "FILEA",
            "FILEF", "FILEI", "FILEE", "FILEH",
        ];

        let cat_order = vec![
            "FILEA", "FILEB", "FILEC", "FILED", "FILEE", "FILEF", "FILEG", "FILEH", "FILEI",
            "FILEJ", "FILEK", "FILEL", "FILEM",
        ];
        let dir_order = example_names.clone();

        let columns_cat = vec![
            vec!["FILEA", "FILEB", "FILEC", "FILED"],
            vec!["FILEE", "FILEF", "FILEG", "FILEH"],
            vec!["FILEI", "FILEJ", "FILEK", "FILEL"],
            vec!["FILEM"],
        ];
        let columns_dir = vec![
            vec!["FILEG", "FILEK", "FILEA", "FILEH"],
            vec!["FILEB", "FILEM", "FILEF"],
            vec!["FILEJ", "FILEC", "FILEI"],
            vec!["FILED", "FILEL", "FILEE"],
        ];
        for name in example_names {
            let bytes = name.as_bytes();
            let entry = match bytes.len() {
                5 => {
                    PrintableEntryFileName {
                        f1: bytes[0],
                        f2: bytes[1],
                        f3: bytes[2],
                        f4: bytes[3],
                        f5: bytes[4],
                        f6: b' ',
                        f7: 0,
                        f8: 0,
                        e1: b' ',
                        e2: b' ',
                        e3: b' '
                    }
                },
                6 => {
                    PrintableEntryFileName {
                        f1: bytes[0],
                        f2: bytes[1],
                        f3: bytes[2],
                        f4: bytes[3],
                        f5: bytes[4],
                        f6: bytes[5],
                        f7: 0,
                        f8: 0,
                        e1: b' ',
                        e2: b' ',
                        e3: b' '
                    }
                },
                _ => panic!("Name must be 5 or 6 characters")
            };
            entries.push(entry);
        }

        // We do not create remaining entries and create directly the catalg wirh few entries
        let catalog = Catalog::try_from(entries.as_slice()).expect("Failed to create test entries");
        let unified_catalog = UnifiedCatalog::from(catalog);

        let sorted_cat = unified_catalog.visible_sorted_entries(CatalogType::Cat);
        let sorted_dir = unified_catalog.visible_sorted_entries(CatalogType::Dir);

        assert_eq!(
            sorted_cat
                .iter()
                .map(|e| e.display_name())
                .collect::<Vec<_>>(),
            cat_order
        );
        assert_eq!(
            sorted_dir
                .iter()
                .map(|e| e.display_name())
                .collect::<Vec<_>>(),
            dir_order
        );

        // Test CAT (sorted alphabetically)
        let cat_result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Cat);
        assert_eq!(cat_result.num_columns(), 4); // 4 columns for Mode2

        // Extract display names from each column
        let col0: Vec<String> = cat_result
            .column(0)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();
        let col1: Vec<String> = cat_result
            .column(1)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();
        let col2: Vec<String> = cat_result
            .column(2)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();
        let col3: Vec<String> = cat_result
            .column(3)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();

        assert_eq!(col0, columns_cat[0]);
        assert_eq!(col1, columns_cat[1]);
        assert_eq!(col2, columns_cat[2]);
        assert_eq!(col3, columns_cat[3]);

        // Test DIR (original order)
        let dir_result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Dir);
        assert_eq!(dir_result.num_columns(), 4); // 4 columns for Mode2

        // Extract display names from each column
        let dir_col0: Vec<String> = dir_result
            .column(0)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();
        let dir_col1: Vec<String> = dir_result
            .column(1)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();
        let dir_col2: Vec<String> = dir_result
            .column(2)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();
        let dir_col3: Vec<String> = dir_result
            .column(3)
            .unwrap()
            .iter()
            .map(|e| e.display_name())
            .collect();

        assert_eq!(dir_col0, columns_dir[0]);
        assert_eq!(dir_col1, columns_dir[1]);
        assert_eq!(dir_col2, columns_dir[2]);
        assert_eq!(dir_col3, columns_dir[3]);

        // Other columns should be empty or have later entries
    }

    #[test]
    fn test_entries_by_mode_and_order_row_major() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog);
        let result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);

        // Mode1 should have 2 columns
        assert_eq!(result.num_columns(), 2);

        // Verify column-major distribution for CAT: entries are distributed column-major
        // Since entries are sorted alphabetically: apple, banana, cher, dog, file05, ...
        // With column-major filling: fill column 0 completely, then column 1
        assert_eq!(result.column(0).unwrap()[0].display_name(), "apple"); // col 0, entry 0
        assert_eq!(result.column(0).unwrap()[1].display_name(), "banana"); // col 0, entry 1
        assert_eq!(result.column(0).unwrap()[2].display_name(), "cher"); // col 0, entry 2
        assert_eq!(result.column(0).unwrap()[3].display_name(), "dog"); // col 0, entry 3
        assert_eq!(result.column(0).unwrap()[4].display_name(), "file05"); // col 0, entry 4

        // Column 1 should have the remaining entries starting from file33
        assert_eq!(result.column(1).unwrap()[0].display_name(), "file33"); // col 1, entry 0
        assert_eq!(result.column(1).unwrap()[1].display_name(), "file34"); // col 1, entry 1
        assert_eq!(result.column(1).unwrap()[2].display_name(), "file35"); // col 1, entry 2
        assert_eq!(result.column(1).unwrap()[3].display_name(), "file36"); // col 1, entry 3
        assert_eq!(result.column(1).unwrap()[4].display_name(), "file37"); // col 1, entry 4
    }

    #[test]
    fn test_entries_by_mode_and_order_column_distribution() {
        // Create a catalog with exactly 64 entries for testing vertical distribution in CAT mode
        let mut entries = Vec::new();

        // Create 64 unique entries (0-63)
        for i in 0..64 {
            let name_char = if i < 26 {
                (b'a' + i as u8) as char
            }
            else if i < 52 {
                (b'A' + (i - 26) as u8) as char
            }
            else {
                (b'0' + (i - 52) as u8) as char
            };
            entries.push(PrintableEntryFileName {
                f1: name_char as u8,
                f2: b' ',
                f3: b' ',
                f4: b' ',
                f5: b' ',
                f6: b' ',
                f7: 0,
                f8: 0,
                e1: b' ',
                e2: b' ',
                e3: b' '
            });
        }

        let catalog = Catalog::try_from(
            entries
                .iter()
                .map(|f| PrintableEntry::from(*f))
                .collect::<Vec<_>>()
                .as_slice()
        )
        .unwrap();
        let unified_catalog = UnifiedCatalog::from(catalog);

        // Test Mode2 (4 columns) with CAT sorting - uses horizontal filling
        let result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Cat);
        assert_eq!(result.num_columns(), 4);

        // With 64 entries in 4 columns, each should have 16 entries
        for col_idx in 0..result.num_columns() {
            assert_eq!(
                result.column(col_idx).unwrap().len(),
                16,
                "Column {} should have 16 entries",
                col_idx
            );
        }

        // Verify we have all 64 unique entries distributed
        let total_entries: usize = (0..result.num_columns())
            .map(|i| result.column(i).unwrap().len())
            .sum();
        assert_eq!(total_entries, 64, "Total entries should be 64");
    }

    #[test]
    fn test_entries_by_mode_and_order_empty_catalog() {
        // Create a catalog with all empty entries
        let mut entries = Vec::new();
        for _ in 0..64 {
            entries.push(PrintableEntryFileName {
                f1: 0xE5,
                f2: 0xE5,
                f3: 0xE5,
                f4: 0xE5,
                f5: 0xE5,
                f6: 0xE5,
                f7: 0xE5,
                f8: 0xE5,
                e1: 0xE5,
                e2: 0xE5,
                e3: 0xE5
            });
        }

        let catalog = Catalog::try_from(
            entries
                .iter()
                .map(|f| PrintableEntry::from(*f))
                .collect::<Vec<_>>()
                .as_slice()
        )
        .unwrap();
        let unified_catalog = UnifiedCatalog::from(catalog);

        // Test CAT mode - all empty entries should be filtered out
        let cat_result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);
        assert_eq!(cat_result.num_columns(), 2);
        // All entries are empty, so no entries should be distributed
        assert_eq!(
            cat_result.column(0).unwrap().len() + cat_result.column(1).unwrap().len(),
            0
        );

        // Test DIR mode - entries should maintain original order
        let dir_result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Dir);
        assert_eq!(dir_result.num_columns(), 2);
        assert_eq!(
            dir_result.column(0).unwrap().len() + dir_result.column(1).unwrap().len(),
            0
        );
    }

    #[test]
    fn test_entries_by_mode_and_order_single_entry() {
        let mut entries = Vec::new();
        entries.push(PrintableEntryFileName {
            f1: b't',
            f2: b'e',
            f3: b's',
            f4: b't',
            f5: 0,
            f6: 0,
            f7: 0,
            f8: 0,
            e1: 1,
            e2: 2,
            e3: 3
        });
        // Fill rest with empty (using 0xe5 pattern for empty entries)
        for _ in 1..64 {
            entries.push(PrintableEntryFileName::empty());
        }

        let catalog = Catalog::try_from(
            entries
                .iter()
                .map(|f| PrintableEntry::from(*f))
                .collect::<Vec<_>>()
                .as_slice()
        )
        .unwrap();
        let unified_catalog = UnifiedCatalog::from(catalog);

        // Test DIR mode - maintains original order with horizontal filling
        let dir_result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Dir);
        assert_eq!(dir_result.num_columns(), 4);

        // Only non-empty entries are distributed, so only 1 entry total
        let total_entries: usize = (0..dir_result.num_columns())
            .map(|i| dir_result.column(i).unwrap().len())
            .sum();
        assert_eq!(total_entries, 1);

        // First entry should be in column 0, row 0
        assert_eq!(
            dir_result.column(0).unwrap()[0].display_name(),
            "test\0\0\0\0.\u{1}\u{2}\u{3}"
        );

        // Test CAT mode - sorts alphabetically, so "test" should be the only entry
        let cat_result =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Cat);
        assert_eq!(cat_result.num_columns(), 4);

        // Only 1 non-empty entry
        let cat_total_entries: usize = (0..cat_result.num_columns())
            .map(|i| cat_result.column(i).unwrap().len())
            .sum();
        assert_eq!(cat_total_entries, 1);

        // "test" should be in column 0 due to vertical filling
        assert_eq!(
            cat_result.column(0).unwrap()[0].display_name(),
            "test\0\0\0\0.\u{1}\u{2}\u{3}"
        );
    }

    #[test]
    fn test_to_char_commands() {
        let entry = PrintableEntryFileName {
            f1: b't',
            f2: b'e',
            f3: b's',
            f4: b't',
            f5: b' ',
            f6: b' ',
            f7: 0,
            f8: 0,
            e1: 1,
            e2: 2,
            e3: 3
        };
        let commands = entry.commands();

        // Should generate some commands
        assert!(commands.as_slice().len() > 0);
    }

    #[test]
    fn test_entries_grid_to_owned() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog);
        let grid =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);

        // Convert to owned
        let owned_grid = grid.to_owned();

        // Should have same structure
        assert_eq!(owned_grid.num_columns(), grid.num_columns());
        assert_eq!(owned_grid.max_num_rows(), grid.max_num_rows());

        // All entries should be owned (Cow::Owned)
        for col in 0..owned_grid.num_columns() {
            for entry in owned_grid.column(col).unwrap() {
                assert!(matches!(entry, Cow::Owned(_)));
            }
        }

        // Original grid should still have borrowed entries
        for col in 0..grid.num_columns() {
            for entry in grid.column(col).unwrap() {
                assert!(matches!(entry, Cow::Borrowed(_)));
            }
        }
    }

    #[test]
    fn test_from_entries_grid_for_catalog() {
        let catalog = Catalog::new(create_test_entries());
        let unified_catalog = UnifiedCatalog::from(catalog.clone());
        let grid =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Cat);

        // Convert back to catalog
        let reconstructed_unified: UnifiedCatalog = grid.into();

        // Should have 64 entries
        let entries: Vec<_> = reconstructed_unified.visible_entries().collect();
        assert_eq!(entries.len(), 64);

        // Create a new grid for comparison
        let comparison_grid =
            unified_catalog.visible_entries_by_mode_and_order(ScreenMode::Mode2, CatalogType::Cat);
        let grid_entries: Vec<String> = comparison_grid
            .entries_display_order()
            .map(|entry| entry.display_name())
            .collect();

        let catalog_entries: Vec<String> = reconstructed_unified
            .visible_entries()
            .map(|entry| entry.display_name())
            .collect();

        // Should match (though catalog might have empty entries padded)
        assert_eq!(grid_entries, catalog_entries);
    }

    #[test]
    fn test_entries_grid_round_trip() {
        let original_catalog = Catalog::new(create_test_entries());
        let original_unified = UnifiedCatalog::from(original_catalog.clone());

        // Convert to grid and back to catalog
        let grid =
            original_unified.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);
        let reconstructed_unified: UnifiedCatalog = grid.into();

        // Should preserve the entries in display order
        let original_display_order: Vec<String> = original_unified
            .visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat)
            .entries_display_order()
            .map(|entry| entry.display_name())
            .collect();

        let reconstructed_display_order: Vec<String> = reconstructed_unified
            .visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat)
            .entries_display_order()
            .map(|entry| entry.display_name())
            .collect();

        assert_eq!(original_display_order, reconstructed_display_order);
    }

    #[test]
    fn test_serial_catalog_builder() {
        use crate::interpret;

        // Create some test commands
        let all_commands = CharCommandList::from(vec![
            CharCommand::Mode(1), /* First command is mode, so first entry will be SequentialFirstWithMode */
            CharCommand::Char(b'H'),
            CharCommand::Char(b'e'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'o'),
            CharCommand::Pen(2),
            CharCommand::Locate(10, 20),
            CharCommand::Char(b'W'),
            CharCommand::Char(b'o'),
            CharCommand::Char(b'r'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'd'),
        ]);
        let nb_commands = all_commands.len();

        for i in 1..=nb_commands {
            let commands = CharCommandList::from(all_commands.as_slice()[0..i].to_vec());

            let builder = SerialCatalogBuilder::new();
            let unified_catalog = builder.build(&commands, ScreenMode::Mode1);

            let expected_nb_entries = match i {
                1..=6 => 1,  // Mode + "Hello" fits in first entry (7 bytes <= 8 bytes capacity)
                7..=10 => 2, /* Pen + Locate + "Wo" fits in second entry (7 bytes = 7 bytes capacity) */
                _ => 3       // Remaining chars need third entry
            };

            // XXX it currently fails but maybe it should not
            assert_eq!(
                unified_catalog.visible_entries().count(),
                expected_nb_entries,
                "Catalog should have {} entries for {} commands",
                expected_nb_entries,
                i
            );

            // Reconstruct commands from catalog
            let reconstructed_commands = unified_catalog.commands_by_mode_and_order_with_params(
                ScreenMode::Mode1,
                CatalogType::Cat,
                false
            );

            // Interpret both to get screen output
            let mut original_interpreter = interpret::Interpreter::new_6128();
            original_interpreter.interpret(&commands, false).unwrap();
            let original_screen = original_interpreter.to_string();

            let mut reconstructed_interpreter = interpret::Interpreter::new_6128();
            reconstructed_interpreter
                .interpret(&reconstructed_commands, false)
                .unwrap();
            let reconstructed_screen = reconstructed_interpreter.to_string();

            // They should produce the same screen output
            assert_eq!(original_screen, reconstructed_screen);
        }
    }

    #[test]
    fn test_serial_catalog_builder_no_mode() {
        use crate::interpret;

        // Create commands without initial mode
        let commands = CharCommandList::from(vec![
            CharCommand::Char(b'H'),
            CharCommand::Char(b'e'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'o'),
            CharCommand::Pen(2),
            CharCommand::Locate(10, 20),
            CharCommand::Char(b'W'),
            CharCommand::Char(b'o'),
            CharCommand::Char(b'r'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'd'),
            CharCommand::Cls,
        ]);

        let builder = SerialCatalogBuilder::new();
        let catalog = builder.build(&commands, ScreenMode::Mode1);

        // Reconstruct commands from catalog
        let reconstructed_commands = catalog.commands_by_mode_and_order_with_params(
            ScreenMode::Mode1,
            CatalogType::Cat,
            false
        );
        // Interpret both to get screen output
        let mut original_interpreter = interpret::Interpreter::new_6128();
        original_interpreter.interpret(&commands, false).unwrap();
        let original_screen = original_interpreter.to_string();

        let mut reconstructed_interpreter = interpret::Interpreter::new_6128();
        reconstructed_interpreter
            .interpret(&reconstructed_commands, false)
            .unwrap();
        let reconstructed_screen = reconstructed_interpreter.to_string();

        // They should produce the same screen output
        assert_eq!(original_screen, reconstructed_screen);
    }

    #[test]
    fn test_serial_catalog_builder_empty() {
        use crate::interpret;

        // Empty commands
        let commands = CharCommandList::from(vec![]);

        let builder = SerialCatalogBuilder::new();
        let unified_catalog = builder.build(&commands, ScreenMode::Mode1);

        // Reconstruct commands from catalog
        let reconstructed_commands =
            unified_catalog.commands_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);

        // input commands are empty
        assert_eq!(commands.as_slice().len(), 0);
        // reconstructed catalog is empty because there are no files
        assert_eq!(unified_catalog.entries.len(), 0);

        // reconstructed commands are NOT empty because of headers
        assert!(reconstructed_commands.as_slice().len() > 0);
    }

    #[test]
    fn catalog_empty() {
        let empty = Catalog::empty();
        assert_eq!(0, empty.entries().count());
    }

    #[test]
    fn catalog_modify() {
        let mut cat = Catalog::empty();
        let entry = PrintableEntryFileName {
            f1: b't',
            f2: b'e',
            f3: b's',
            f4: b't',
            f5: b' ',
            f6: b' ',
            f7: 0,
            f8: 0,
            e1: 1,
            e2: 2,
            e3: 3
        };
        cat.add(PrintableEntry::from(entry.clone())).unwrap();
        assert_eq!(1, cat.entries().count());
        assert_eq!(
            entry.display_name(),
            cat.entries().next().unwrap().fname.display_name()
        );
    }

    #[test]
    fn individually_build_entries_from_commands() {
        let cmd_mode1 = CharCommand::Mode(1).bytes();
        let cmd_gfx_mode = CharCommand::GraphicsInkMode(b'.').bytes();
        let nop = CharCommand::Nop.first_byte();
        let dis = CharCommand::DisableVdu.first_byte();
        let locate = CharCommand::Locate(1, 1).bytes();
        let pen = CharCommand::Pen(2).bytes();
        let paper = CharCommand::Paper(3).bytes();

        // test the first entry with mode
        // SequentialFirstWithMode has: Mode (2 bytes) + Any(4) + Any(2) = 8 bytes real_size
        // So: Mode + 6 single-byte commands
        let entry = EntryKind::SequentialFirstWithMode(DotHiding::AsModeParameter).build_entry(
            &[
                CharCommand::Mode(1),
                CharCommand::Nop,
                CharCommand::Nop,
                CharCommand::Nop,
                CharCommand::Nop,
                CharCommand::Nop,
                CharCommand::Nop
            ],
            None
        );

        assert_eq!(
            entry,
            PrintableEntryFileName {
                f1: b' ',         // index placeholder
                f2: cmd_mode1[0], // EOT
                f3: cmd_mode1[1], // mode value
                f4: nop,
                f5: nop,
                f6: nop,
                f7: nop,
                f8: cmd_gfx_mode[0],
                e1: nop,
                e2: nop,
                e3: dis
            }
        );

        let entry = EntryKind::SequentialFirstWithMode(DotHiding::AsModeParameter).build_entry(
            &[
                CharCommand::Mode(1),
                CharCommand::Char(b'A'),
                CharCommand::Char(b'B'),
                CharCommand::Char(b'C'),
                CharCommand::Char(b'D'),
                CharCommand::Char(b'E'),
                CharCommand::Char(b'F')
            ],
            None
        );

        assert_eq!(
            entry,
            PrintableEntryFileName {
                f1: b' ',         // index placeholder
                f2: cmd_mode1[0], // EOT
                f3: cmd_mode1[1], // mode value
                f4: b'A',
                f5: b'B',
                f6: b'C',
                f7: b'D',
                f8: cmd_gfx_mode[0],
                e1: b'E',
                e2: b'F',
                e3: dis
            }
        );
    }

    #[test]
    fn test_builder_entries_creation() {
        let commands = CharCommandList::from(vec![
            CharCommand::Mode(1),
            CharCommand::Char(b'H'),
            CharCommand::Char(b'e'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'o'),
            CharCommand::Pen(2),
            CharCommand::Locate(10, 20),
            CharCommand::Char(b'W'),
            CharCommand::Char(b'o'),
            CharCommand::Char(b'r'),
            CharCommand::Char(b'l'),
            CharCommand::Char(b'd'),
            CharCommand::Cls,
        ]);

        let builder = SerialCatalogBuilder::new();
        let entries = builder.build_entries(&commands, ScreenMode::Mode1);
        let expected_entries = &[
            // First entry: SequentialFirstWithMode contains Mode + H,e,l,l,o
            // f1 is space (index placeholder), f2=EOT, f3=mode_value
            PrintableEntryFileName {
                f1: b' ',                               // Space: index placeholder
                f2: CharCommand::Mode(1).first_byte(),  // EOT
                f3: CharCommand::Mode(1).second_byte(), // mode value
                f4: b'H',
                f5: b'e',
                f6: b'l',
                f7: b'l',
                f8: CharCommand::GraphicsInkMode(b'.').first_byte(),
                e1: b'o', // Only one byte fills Any(2) slot
                e2: 0,    // Padding (Pen command goes to next entry)
                e3: CharCommand::DisableVdu.first_byte()
            },
            // Second entry: SequentialBasic contains Pen + Locate + W,o
            // f1 is the index character (calculated by builder)
            PrintableEntryFileName {
                f1: b'!',                                      // Index 33 (0x21)
                f2: CharCommand::EnableVdu.first_byte(),       // ACK
                f3: CharCommand::Pen(2).first_byte(),          // SI
                f4: CharCommand::Pen(2).second_byte(),         // 2
                f5: CharCommand::Locate(10, 20).first_byte(),  // US
                f6: CharCommand::Locate(10, 20).second_byte(), // 11
                f7: CharCommand::Locate(10, 20).third_byte(),  // 21
                f8: CharCommand::GraphicsInkMode(b'.').first_byte(),
                e1: b'W',
                e2: b'o',
                e3: CharCommand::DisableVdu.first_byte()
            },
            // Third entry: SequentialBasic contains r,l,d,Cls + padding
            PrintableEntryFileName {
                f1: b'"', // Index 34 (0x22)
                f2: CharCommand::EnableVdu.first_byte(),
                f3: b'r',
                f4: b'l',
                f5: b'd',
                f6: CharCommand::Cls.first_byte(),
                f7: CharCommand::Nop.first_byte(),
                f8: CharCommand::GraphicsInkMode(b'.').first_byte(),
                e1: CharCommand::Nop.first_byte(),
                e2: CharCommand::Nop.first_byte(),
                e3: CharCommand::DisableVdu.first_byte()
            }
        ];

        for (obtained, expected) in entries.iter().zip(expected_entries.iter()) {
            assert_eq!(obtained, expected);
        }

        // Further assertions can be added based on expected entries
    }

    #[test]
    fn test_raw_printable_entries() {
        let fname = PrintableEntryFileName::new("TEST.TXT");
        let size2 = PrintableEntry::artificial(fname, 2);
        let size16 = PrintableEntry::artificial(fname, 16);
        let size17 = PrintableEntry::artificial(fname, 17);
        let size33 = PrintableEntry::artificial(fname, 33);

        assert_eq!(size2.len(), 1);
        assert_eq!(size16.len(), 1);
        assert_eq!(size17.len(), 2);
        assert_eq!(size33.len(), 3);
    }

    #[test]
    fn test_unified_catalog_mode1() {
        let mut catalog = UnifiedCatalog::empty();
        assert_eq!(0, catalog.visible_entries().count());

        let fname = PrintableEntryFileName::new("TEST.TXT");
        assert_eq!(&fname.all_generated_bytes(), b"TEST    .TXT");
        let entry = PrintableEntry::artificial(fname, 5);
        let entry: UnifiedPrintableEntry = entry.into();
        assert_eq!(5, entry.size_kb());
        catalog.push(entry.clone()).unwrap();
        assert_eq!(1, catalog.visible_entries().count());

        let err = catalog.push(entry.into());
        assert!(err.is_err());
        assert_eq!(1, catalog.visible_entries().count());

        let fname = PrintableEntryFileName::new("TEST2.TXT");
        assert_eq!(&fname.all_generated_bytes(), b"TEST2   .TXT");
        let entry = PrintableEntry::artificial(fname, 20);
        let entry: UnifiedPrintableEntry = entry.into();
        assert_eq!(20, entry.size_kb());
        catalog.push(entry).unwrap();
        assert_eq!(2, catalog.visible_entries().count());

        let fname = PrintableEntryFileName::new("ABCDEFGH.IJK");
        assert_eq!(&fname.all_generated_bytes(), b"ABCDEFGH.IJK");
        let entry = PrintableEntry::artificial(fname, 16);
        let entry: UnifiedPrintableEntry = entry.into();
        assert_eq!(16, entry.size_kb());
        catalog.push(entry).unwrap();
        assert_eq!(3, catalog.visible_entries().count());

        assert_eq!(41, catalog.size_kb());

        let cat = catalog.visible_entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Cat);
        assert_eq!(2, cat.num_columns());
        assert_eq!(2, cat.max_num_rows());
        assert_eq!(cat.get(0, 0).unwrap().display_name(), "ABCDEFGH.IJK");
        assert_eq!(cat.get(0, 1).unwrap().display_name(), "TEST2   .TXT");
        assert_eq!(cat.get(1, 0).unwrap().display_name(), "TEST    .TXT");

        let display_commands = cat.commands();
        let mut interpreter = crate::interpret::Interpreter::new_6128();
        interpreter.interpret(&display_commands, true).unwrap();
        let screen_output = interpreter.to_string();
        println!("Screen Output:\n{}", screen_output);

        // let dir = catalog.entries_by_mode_and_order(ScreenMode::Mode1, CatalogType::Dir);
        // assert_eq!(3, dir.num_columns());
        // assert_eq!(2, dir.max_num_rows());
    }
}
