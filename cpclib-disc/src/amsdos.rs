use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path};

use arrayref::array_ref;
use cpclib_common::bitfield::Bit;
use delegate::delegate;
use thiserror::Error;

use crate::disc::Disc;
use crate::edsk::Head;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum AmsdosError {
    #[error("No more entries available.")]
    NoEntriesAvailable,

    #[error("No more blocs available.")]
    NoBlocAvailable,

    #[error("File larger than 64kb")]
    FileLargerThan64Kb,

    #[error("Invalid header")]
    InvalidHeader,

    #[error("IO error: {0}")]
    IO(String),

    #[error("Various error: {0}")]
    Various(String),

    #[error("File name error: {}", msg)]
    WrongFileName { msg: String },

    #[error("File `{0}` already present in disc")]
    FileAlreadyExists(String),

    #[error("File `{0}` not present in disc")]
    FileDoesNotExist(String)
}

impl From<std::io::Error> for AmsdosError {
    fn from(err: std::io::Error) -> Self {
        AmsdosError::IO(err.to_string())
    }
}

impl From<String> for AmsdosError {
    fn from(err: String) -> Self {
        AmsdosError::Various(err)
    }
}

/// The AmsdosFileName structure is used to encode several informations
/// - the user
/// - the filename (up to 8 chars)
/// - the extension (up to 3 chars)
/// It does not contain property information
#[derive(Clone, Copy)]
pub struct AmsdosFileName {
    user: u8,
    name: [u8; 8],
    extension: [u8; 3]
}

impl std::fmt::Debug for AmsdosFileName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}.{}", self.user(), self.name(), self.extension())
    }
}

impl PartialEq for AmsdosFileName {
    fn eq(&self, other: &Self) -> bool {
        self.user == other.user
            && self.name().to_uppercase() == other.name().to_uppercase()
            && self.extension().to_uppercase() == other.extension().to_uppercase()
    }
}

#[allow(missing_docs)]
impl AmsdosFileName {
    pub fn filename_header_format(&self) -> &[u8; 8] {
        // let mut content = [' ' as u8;8];
        // println!("*{:?}*", self.name);
        // for (i, c) in self.name.as_bytes().iter().enumerate() {
        // content[i] = *c;
        // }
        // content
        &self.name
    }

    pub fn extension_header_format(&self) -> &[u8; 3] {
        // let mut content = [' ' as u8;3];
        // for (i, c) in self.extension.as_bytes().iter().enumerate() {
        // content[i] = *c;
        // }
        // content
        &self.extension
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self::from_entry_format(array_ref!(slice, 0, 12))
    }

    /// Create an amsdos filename from a catalog entry buffer
    pub fn from_entry_format(buffer: &[u8; 12]) -> Self {
        let user: u8 = buffer[0];
        let name: [u8; 8] = *array_ref!(buffer, 1, 8);
        // Remove bit 7 of each char
        let mut extension: [u8; 3] = *array_ref!(buffer, 9, 3);
        extension.iter_mut().for_each(|b| *b &= 0b0111_1111);

        Self {
            user,
            name,
            extension
        }
    }

    /// Build a filename compatible with the catalog entry format
    pub fn to_entry_format(&self, system: bool, read_only: bool) -> [u8; 12] {
        let mut buffer = [0; 12];

        buffer[0] = self.user;
        buffer[1..9].copy_from_slice(self.filename_header_format().as_ref());
        buffer[9..].copy_from_slice(self.extension_header_format().as_ref());

        if system {
            buffer[10] += 0b1000_0000;
        }
        if read_only {
            buffer[9] += 0b1000_0000;
        }

        buffer
    }

    pub fn user(&self) -> u8 {
        self.user
    }

    pub fn set_user(&mut self, user: u8) {
        self.user = user;
    }

    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name)
            .into_owned()
            .trim()
            .to_owned()
    }

    pub fn extension(&self) -> String {
        String::from_utf8_lossy(&self.extension)
            .into_owned()
            .trim()
            .to_owned()
    }

    pub fn filename(&self) -> String {
        format!("{}.{}", self.name(), self.extension())
    }

    pub fn filename_with_user(&self) -> String {
        format!("{}:{}.{}", self.user(), self.name(), self.extension())
    }

    pub fn set_filename<S: AsRef<str>>(&mut self, filename: S) {
        let filename = filename.as_ref();
        if let Some(idx) = filename.find('.') {
            self.set_name(&filename[0..idx]);
            self.set_extension(&filename[idx + 1..filename.len()])
        }
        else {
            self.set_name(filename);
            self.set_extension("");
        }
    }

    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        let name = name.as_ref().as_bytes();
        for idx in 0..8 {
            self.name[idx] = if idx < name.len() {
                *name.get(idx).unwrap()
            }
            else {
                b' '
            };
        }
    }

    pub fn set_extension<S: AsRef<str>>(&mut self, extension: S) {
        let extension = extension.as_ref().as_bytes();
        for idx in 0..3 {
            self.extension[idx] = if idx < extension.len() {
                *extension.get(idx).unwrap()
            }
            else {
                b' '
            };
        }
    }

    // A filename can only contains the chars a-z A-Z 0-9 ! " # $ & ' +  @ ^ ' } {
    // when typed in the CPC. However, onmy upper case is used in the discs. So lowercase is not allowedthere
    pub fn is_valid(&self) -> bool {
        self.name().bytes().all(Self::is_valid_char)
            && self.extension().bytes().all(Self::is_valid_char)
            && self.user() <= 16
    }

    pub fn is_valid_char(char: u8) -> bool {
        // (char >= 'a' as u8 && char <= 'z' as u8) ||
        char.is_ascii_uppercase()
            || char.is_ascii_digit()
            || char == b'!'
            || char == b'"'
            || char == b'#'
            || char == b'$'
            || char == b'&'
            || char == b'+'
            || char == b'@'
            || char == b'^'
            || char == b'\''
            || char == b'{'
            || char == b'}'
            || char == b' ' // by definition ' ' is already used to fill space
    }

    // Build a AmsdosFileName ensuring the case is correct
    pub fn new_correct_case<S1, S2>(
        user: u8,
        filename: S1,
        extension: S2
    ) -> Result<Self, AmsdosError>
    where
        S1: AsRef<str>,
        S2: AsRef<str>
    {
        Self::new_incorrect_case(
            user,
            &filename.as_ref().to_ascii_uppercase(),
            &extension.as_ref().to_ascii_uppercase()
        )
    }

    // Build a AmsdosFileName without checking case
    pub fn new_incorrect_case(
        user: u8,
        filename: &str,
        extension: &str
    ) -> Result<Self, AmsdosError> {
        let filename = filename.trim();
        let extension = extension.trim();

        // TODO check the user validity
        if filename.len() > 8 {
            return Err(AmsdosError::WrongFileName {
                msg: format!("{} should use at most 8 chars", filename)
            });
        }

        if extension.len() > 3 {
            return Err(AmsdosError::WrongFileName {
                msg: format!("{} should use at most 3 chars", extension)
            });
        }

        if filename.to_ascii_uppercase() != filename.to_ascii_uppercase() {
            return Err(AmsdosError::WrongFileName {
                msg: format!("{} contains non ascii characters", filename)
            });
        }

        if extension.to_ascii_uppercase() != extension.to_ascii_uppercase() {
            return Err(AmsdosError::WrongFileName {
                msg: format!("{} contains non ascii characters", extension)
            });
        }

        let name = {
            let mut encoded_filename = [b' '; 8];
            for (idx, c) in filename.as_bytes().iter().enumerate() {
                encoded_filename[idx] = *c
            }
            encoded_filename
        };

        let extension = {
            let mut encoded_extension = [b' '; 3];
            for (idx, c) in extension.as_bytes().iter().enumerate() {
                encoded_extension[idx] = *c
            }
            encoded_extension
        };

        // TODO see if upercase is needed
        Ok(Self {
            user,
            name,
            extension
        })
    }
}

// TODO use tryfrom asap
impl TryFrom<&str> for AmsdosFileName {
    type Error = AmsdosError;

    /// Make a filename conversion by considering the following format is used: user:name.extension
    fn try_from(content: &str) -> Result<Self, Self::Error> {
        let (user, rest) = match content.find(':') {
            None => (0, content),
            Some(1) => {
                (
                    u8::from_str_radix(&content[..1], 10).unwrap_or_default(),
                    &content[2..]
                )
            },
            _ => unreachable!()
        };

        let (filename, extension) = match rest.find('.') {
            None => (rest, ""),
            Some(idx) => (&rest[..idx], &rest[(idx + 1)..])
        };

        Self::new_correct_case(user, filename, extension)
    }
}

#[derive(Clone, Copy)]
/// Encodes the amsdos file type
pub enum AmsdosFileType {
    /// Basic file type
    Basic = 0,
    /// Protected binary file type
    Protected = 1,
    /// Binary file type
    Binary = 2
}

impl TryFrom<u8> for AmsdosFileType {
    type Error = AmsdosError;

    fn try_from(val: u8) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(AmsdosFileType::Basic),
            1 => Ok(AmsdosFileType::Protected),
            2 => Ok(AmsdosFileType::Binary),
            _ => {
                Err(AmsdosError::Various(format!(
                    "{} is not a valid file type",
                    val
                )))
            },
        }
    }
}

impl std::fmt::Debug for AmsdosFileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            AmsdosFileType::Basic => "Basic",
            AmsdosFileType::Protected => "Protected",
            AmsdosFileType::Binary => "Binary"
        };
        write!(f, "{}", repr)
    }
}
/// Encode the index of a bloc
#[derive(Debug, Copy, Clone, Ord, Eq, Default)]
pub enum BlocIdx {
    /// The block is not used
    #[default]
    Empty,
    /// The block is deleted
    Deleted, // TODO find a real name
    /// Index of a real bloc
    Index(std::num::NonZeroU8)
}

impl From<u8> for BlocIdx {
    fn from(val: u8) -> Self {
        match val {
            0 => BlocIdx::Empty,
            0xE5 => BlocIdx::Deleted,
            val => BlocIdx::Index(unsafe { std::num::NonZeroU8::new_unchecked(val) })
        }
    }
}

impl Into<u8> for &BlocIdx {
    fn into(self) -> u8 {
        match self {
            BlocIdx::Empty => 0,
            BlocIdx::Deleted => 0xE5,
            BlocIdx::Index(ref val) => val.get()
        }
    }
}

impl PartialOrd for BlocIdx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let a: u8 = self.into();
        let b: u8 = other.into();
        a.partial_cmp(&b)
    }
}

impl PartialEq for BlocIdx {
    fn eq(&self, other: &Self) -> bool {
        let a: u8 = self.into();
        let b: u8 = other.into();
        a == b
    }
}

#[allow(missing_docs)]
impl BlocIdx {
    pub fn is_valid(self) -> bool {
        match self {
            BlocIdx::Index(_) => true,
            _ => false
        }
    }

    pub fn value(self) -> u8 {
        (&self).into()
    }

    /// only valid for a valid block
    #[allow(clippy::cast_possible_truncation)]
    pub fn track(self) -> u8 {
        ((u16::from(self.value()) << 1) / 9) as u8
    }

    /// only valid for a valid block
    #[allow(clippy::cast_possible_truncation)]
    pub fn sector(self) -> u8 {
        ((u16::from(self.value()) << 1) % 9) as u8
    }
}

// http://www.cpc-power.com/cpcarchives/index.php?page=articles&num=92
#[derive(Debug, Clone, Copy, PartialEq)]
/// Represents an entry in the Amsdos Catalog
pub struct AmsdosEntry {
    /// Location of the entry in the catalog
    pub(crate) idx: u8,
    /// Name of the file
    pub(crate) file_name: AmsdosFileName,
    pub(crate) read_only: bool,
    pub(crate) system: bool,
    pub(crate) num_page: u8,
    /// Encoded page size
    pub(crate) page_size: u8,
    /// list of block indexes
    pub(crate) blocs: [BlocIdx; 16]
}

impl std::fmt::Display for AmsdosEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fname = self.amsdos_filename().filename_with_user();
        let size = self.used_space();

        write!(f, "{} {}K", fname, size)
    }
}

#[allow(missing_docs)]
impl AmsdosEntry {
    delegate! {
        to self.file_name {
            pub fn user(&self) -> u8;
            pub fn set_user(&mut self, user: u8);

            pub fn name(&self) -> String;
            pub fn extension(&self) -> String;
            pub fn filename(&self) -> String;
            pub fn filename_with_user(&self) -> String;

            pub fn set_filename<S:AsRef<str>>(&mut self, filename: S);
            pub fn set_name<S:AsRef<str>>(&mut self, name: S);
            pub fn set_extension<S:AsRef<str>>(&mut self, extension: S);
        }
    }

    /// Provide the size, in Kb, eaten by the file on disc
    pub fn used_space(&self) -> usize {
        (self.nb_blocs() * DATA_SECTOR_SIZE * 2) / 1024
    }

    /// Check if the given filename corresponds to the entry
    pub fn belongs_to(&self, filename: &AmsdosFileName) -> bool {
        &self.file_name == filename
    }

    pub fn amsdos_filename(&self) -> &AmsdosFileName {
        &self.file_name
    }

    pub fn from_slice(idx: u8, slice: &[u8]) -> Self {
        Self::from_buffer(idx, array_ref!(slice, 0, 32))
    }

    /// Create the entry from its 32 bytes
    pub fn from_buffer(idx: u8, buffer: &[u8; 32]) -> Self {
        Self {
            idx,
            file_name: AmsdosFileName::from_entry_format(array_ref!(buffer, 0, 12)),
            read_only: buffer[1 + 8].bit(7),
            system: buffer[1 + 8 + 1].bit(7),
            num_page: buffer[12],
            page_size: buffer[15],
            blocs: {
                let blocs = buffer[16..]
                    .iter()
                    .map(|&b| BlocIdx::from(b))
                    .collect::<Vec<BlocIdx>>();
                let mut array_blocs = [BlocIdx::default(); 16];
                array_blocs[..16].clone_from_slice(&blocs[..16]);
                array_blocs
            }
        }
    }

    /// Returns the list of used blocs by tis entry
    pub fn used_blocs(&self) -> &[BlocIdx] {
        &self.blocs[..(self.nb_blocs())]
    }

    /// Compute the real number of blocs to read
    pub fn nb_blocs(&self) -> usize {
        (self.page_size as usize + 7) >> 3
    }

    /// Set the number of blocs
    pub fn set_blocs(&mut self, blocs: &[BlocIdx]) {
        let mut nb_blocs = 0;
        for idx in 0..16 {
            self.blocs[idx] = if blocs.len() > idx {
                nb_blocs += 1;
                blocs[idx]
            }
            else {
                BlocIdx::Empty
            }
        }

        self.page_size = (nb_blocs << 3) - 7;
    }

    pub fn as_bytes(&self) -> [u8; 32] {
        let mut bytes = [0; 32];
        bytes[0..12].copy_from_slice(
            self.file_name
                .to_entry_format(self.system, self.read_only)
                .as_ref()
        );
        bytes[12] = self.num_page;
        bytes[15] = self.page_size;
        bytes[16..].copy_from_slice(
            &self
                .blocs
                .iter()
                .map(|b| -> u8 { b.into() })
                .collect::<Vec<u8>>()
        );

        bytes
    }

    pub fn len() -> usize {
        32
    }

    pub fn is_erased(&self) -> bool {
        self.file_name.user() == 0xE5
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn is_system(&self) -> bool {
        self.system
    }

    pub fn set_system(&mut self) {
        self.system = true;
    }

    pub fn set_read_only(&mut self) {
        self.read_only = true;
    }

    pub fn unset_system(&mut self) {
        self.system = false;
    }

    pub fn unset_read_only(&mut self) {
        self.read_only = false;
    }

    pub fn set_num_page(&mut self, num: u8) {
        self.num_page = num;
    }

    pub fn page_size(&self) -> u8 {
        self.page_size
    }

    pub fn set_page_size(&mut self, size: u8) {
        self.page_size = size;
    }

    pub fn format(&self) -> String {
        format!(
            "{}:{}.{}",
            self.file_name.user,
            self.file_name.name(),
            self.file_name.extension()
        )
    }

    fn track_and_sector<D: Disc>(&self, disc: &D, head: Head) -> (u8, u8) {
        let min_sect = disc.global_min_sector(head);
        let sector_id = (self.idx >> 4) + min_sect;
        let track = if min_sect == 0x41 {
            2
        }
        else if min_sect == 1 {
            1
        }
        else {
            0
        };

        (track, sector_id)
    }
}

/// Encode the catalog of an existing disc
#[derive(PartialEq)]
pub struct AmsdosEntries {
    /// List of entried in the catalog
    entries: Vec<AmsdosEntry>
}

impl std::fmt::Debug for AmsdosEntries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for entry in self.used_entries() {
            write!(f, "{:?}", entry)?;
        }
        Ok(())
    }
}

/// Encode a file in the catalog. This file can be represented by several entries
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct AmsdosCatalogEntry {
    track: u8,
    sector: u8,
    file_name: AmsdosFileName,
    read_only: bool,
    system: bool,
    /// The indices, in the real catalog, of the entries that represent this one
    entries_idx: Vec<u8>,
    blocs: Vec<BlocIdx>
}

impl AmsdosCatalogEntry {
    /// Return the file name component of the entry
    pub fn file_name(&self) -> &AmsdosFileName {
        &self.file_name
    }
}

impl From<(u8, u8, AmsdosEntry)> for AmsdosCatalogEntry {
    fn from(e: (u8, u8, AmsdosEntry)) -> Self {
        let (track, sector, e) = e;
        Self {
            track,
            sector,
            file_name: e.file_name,
            read_only: e.read_only,
            system: e.system,
            entries_idx: vec![e.idx],
            blocs: e
                .blocs
                .iter()
                .filter_map(|b| {
                    if b.is_valid() {
                        Some(*b)
                    }
                    else {
                        None
                    }
                })
                .collect()
        }
    }
}

#[allow(missing_docs)]
impl AmsdosCatalogEntry {
    fn merge_entries(e1: &Self, e2: &Self) -> Self {
        assert_eq!(e1.file_name, e2.file_name);

        Self {
            track: e1.track.min(e2.track),
            sector: e1.sector.min(e2.sector),
            file_name: e1.file_name,
            read_only: e1.read_only,
            system: e1.system,
            entries_idx: {
                let mut idx = e1.entries_idx.clone();
                idx.extend_from_slice(&e2.entries_idx);
                idx
            },
            blocs: {
                let mut blocs = e1.blocs.clone();
                blocs.extend_from_slice(&e2.blocs);
                blocs
            }
        }
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn system(&self) -> bool {
        self.system
    }

    pub fn blocs(&self) -> &[BlocIdx] {
        &self.blocs
    }

    /// Size in kilobytes
    pub fn size(&self) -> usize {
        self.blocs.len() * 512 * 2 / 1024
    }
}

/// The AmsdosCatalog represents the catalog of a disc. It contains only valid entries and merge common ones
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct AmsdosCatalog {
    entries: Vec<AmsdosCatalogEntry>
}

#[allow(missing_docs)]
impl From<&AmsdosEntries> for AmsdosCatalog {
    fn from(entries: &AmsdosEntries) -> Self {
        let mut novel: Vec<AmsdosCatalogEntry> = Vec::new();

        for current_entry in entries.without_erased_entries().map(|e| {
            AmsdosCatalogEntry::from((entries.track(e).unwrap(), entries.sector(e).unwrap(), *e))
        }) {
            let mut added = false;
            for entry in &mut novel {
                if entry.file_name == current_entry.file_name {
                    *entry = AmsdosCatalogEntry::merge_entries(entry, &current_entry);
                    added = true;
                    break;
                }
            }

            if !added {
                novel.push(current_entry.clone())
            }
        }

        Self { entries: novel }
    }
}

impl std::ops::Index<usize> for AmsdosCatalog {
    type Output = AmsdosCatalogEntry;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.entries[idx]
    }
}
#[allow(missing_docs)]
impl AmsdosCatalog {
    /// Returns the number of entries in the catalog
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the catalog is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create an alphabetically sorted version of the catalog
    pub fn sorted_alphabetically(&self) -> Self {
        let mut copy: Self = self.clone();
        copy.entries
            .sort_by_key(|entry| entry.file_name().to_entry_format(false, false));
        copy
    }

    /// Create a physically sorted version of the catalog
    pub fn sorted_physically(&self) -> Self {
        let mut copy: Self = self.clone();
        copy.entries
            .sort_by_key(|entry| (entry.track, entry.sector));
        copy
    }

    /// Create a physically and alphabetically sorted version of the catalog
    pub fn sorted_physically_and_alphabetically(&self) -> Self {
        let mut copy: Self = self.clone();
        copy.entries.sort_by_key(|entry| {
            (
                entry.track,
                entry.sector,
                entry.file_name().to_entry_format(false, false)
            )
        });
        copy
    }
}

#[allow(missing_docs)]
impl AmsdosEntries {
    /// Generate a catalog that is more user friendly
    pub fn to_amsdos_catalog(&self) -> AmsdosCatalog {
        AmsdosCatalog::from(self)
    }

    /// Return the index of the entry
    pub fn entry_index(&self, entry: &AmsdosEntry) -> Option<usize> {
        (0..self.entries.len()).find(|&idx| &self.entries[idx] == entry)
    }

    /// Return the track that contains the entry
    pub fn track(&self, entry: &AmsdosEntry) -> Option<u8> {
        self.entry_index(entry).map(|idx| (4 * idx / 64) as u8)
    }

    pub fn sector(&self, entry: &AmsdosEntry) -> Option<u8> {
        match self.entry_index(entry) {
            Some(idx) => {
                let idx = idx % 16;
                Some([0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9][idx / 16])
            },
            _ => None
        }
    }

    pub fn get_entry_mut(&mut self, idx: usize) -> &mut AmsdosEntry {
        &mut self.entries[idx]
    }

    /// Generate a binary version that can be used to export the catalog
    pub fn as_bytes(&self) -> [u8; 64 * 32] {
        let mut content = [0; 64 * 32];
        for i in 0..64 {
            content[i * 32..(i + 1) * 32].copy_from_slice(&self.entries[i].as_bytes());
        }
        content
    }

    /// Manually create the catalog from a byte slice. Usefull to manipulate catarts
    pub fn from_slice(slice: &[u8]) -> Self {
        let mut entries = Vec::new();
        for i in 0..64 {
            entries.push(AmsdosEntry::from_slice(
                i as _,
                &slice[i * 32..(i + 1) * 32]
            ))
        }
        Self { entries }
    }

    /// Returns all the entries of the catalog
    pub fn all_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
        self.entries.iter()
    }

    /// Returns an iterator on the visible entries : they are not erased and are not system entries
    pub fn visible_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
        self.entries
            .iter()
            .filter(|&entry| !entry.is_erased() && !entry.is_system())
    }

    /// Returns an iterator on the entries not erased
    pub fn without_erased_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
        self.entries.iter().filter(|&entry| !entry.is_erased())
    }

    pub fn used_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
        self.entries
            .iter()
            .filter(|&entry| entry.page_size > 0 && !entry.is_erased())
    }

    /// Returns entries erased
    pub fn free_entries(&self) -> impl Iterator<Item = &AmsdosEntry> {
        self.entries
            .iter()
            .filter(|&entry| entry.is_erased() || entry.page_size == 0)
    }

    /// Return all the entries that correspond to a given file
    pub fn for_file(&self, filename: &AmsdosFileName) -> impl Iterator<Item = &AmsdosEntry> {
        let filename: AmsdosFileName = *filename;
        self.entries
            .iter()
            .filter(move |&entry| entry.page_size > 0 && entry.belongs_to(&filename))
    }

    /// Returns one available entry or None if there is no entry
    pub fn one_empty_entry(&self) -> Option<&AmsdosEntry> {
        self.free_entries().next()
    }

    /// Returns the blocs that are not referenced in the catalog.
    /// Bloc used in erased files are returned (so they may be broken)
    pub fn available_blocs(&self) -> impl Iterator<Item = BlocIdx> {
        // first 2 blocs are not available if we trust idsk. So  I do the same

        let used_blocs = self
            .used_entries()
            .flat_map(|e| e.blocs.iter())
            .copied()
            .collect::<std::collections::BTreeSet<BlocIdx>>();

        (2..=255)
            .map(BlocIdx::from)
            .filter(|&v| v.is_valid()) // TODO I'm pretty sure there is womething wrong there with the erased value
            .filter(move |b| !used_blocs.contains(b))
    }

    /// Returns one available bloc or None if there is no entry
    pub fn one_empty_bloc(&self) -> Option<BlocIdx> {
        self.available_blocs().next()
    }
}

#[allow(unused)]
const DIRECTORY_SIZE: usize = 64;
#[allow(unused)]
const DATA_SECTORS: [u8; 9] = [0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9];
#[allow(unused)]
const DATA_NB_RECORDS_PER_TRACK: u8 = 36;
#[allow(unused)]
const DATA_BLOCK_SHIFT: u8 = 3;
#[allow(unused)]
const DATA_BLOCK_MASK: u8 = 7;
#[allow(unused)]
const DATA_EXTENT_MASK: u8 = 0;
#[allow(unused)]
const DATA_NB_BLOCKS: u8 = 180;
#[allow(unused)]
const DATA_TWO_DIRECTORY_BLOCKS: u8 = 0x00C0;
#[allow(unused)]
const DATA_SIZE_OF_CHECKSUM_VECTOR: u8 = 16;
#[allow(unused)]
const DATA_RESERVED_TRACK: u8 = 0;
#[allow(unused)]
const DATA_FIRST_SECTOR_NUMBER: u8 = DATA_SECTORS[0];
#[allow(unused)]
const DATA_SECTORS_PER_TRACK: u8 = 9;
#[allow(unused)]
const DATA_GAP_LENGTH_READ_WRITE: u8 = 42;
#[allow(unused)]
const DATA_GAP_LENGTH_FORMAT: u8 = 82;
#[allow(unused)]
const DATA_FILLER_BYTE: u8 = 0xE9;
#[allow(unused)]
const DATA_LOG2_SECTOR_SIZE_MINUS_SEVEN: u8 = 2;
#[allow(unused)]
const DATA_RECORDS_PER_TRACK: u8 = 4;
#[allow(unused)]
const DATA_SECTOR_SIZE: usize = 512;

/// Minimal information needed to access to twe two sectors of a given bloc
#[allow(missing_docs)]
#[derive(Debug)]
struct BlocAccessInformation {
    track1: u8,
    sector1_id: u8,
    track2: u8,
    sector2_id: u8
}

/// http://cpctech.cpc-live.com/docs/manual/s968se09.pdf
/// Current implementatin only focus on DATA format
#[allow(missing_docs)]
#[derive(Debug)]
pub struct AmsdosManagerMut<'dsk, D: Disc> {
    disc: &'dsk mut D,
    head: Head
}

#[derive(Clone, Copy, Debug)]
pub enum AmsdosAddBehavior {
    // Raise an error
    FailIfPresent,
    // Only remove from the table
    ReplaceIfPresent,
    // Remove from the table AND of the sectors
    ReplaceAndEraseIfPresent,
    // Rename with .BAK
    BackupIfPresent
}

impl<'dsk, 'mng: 'dsk, D: Disc> AmsdosManagerMut<'dsk, D> {
    pub fn new_from_disc<S: Into<Head>>(disc: &'dsk mut D, head: S) -> Self {
        Self {
            disc,
            head: head.into()
        }
    }

    pub fn dsk_mut(&mut self) -> &mut D {
        &mut self.disc
    }

    fn erase_entry(&mut self, entry: &AmsdosEntry) {
        let (track, sector_id) = entry.track_and_sector(self.disc, self.head);

        let mut sector = self
            .disc
            .sector_read_bytes(self.head, track, sector_id)
            .unwrap();
        let idx_in_sector: usize = ((entry.idx & 15) << 5) as usize;
        let bytes = &mut (sector[idx_in_sector..(idx_in_sector + AmsdosEntry::len())]);
        bytes.fill(0xE5);
        self.disc
            .sector_write_bytes(self.head, track, sector_id, &sector)
            .unwrap();
    }

    /// Write the entry information on disc AFTER the sectors has been set up.
    /// Panic if dsk is invalid
    /// Still stolen to iDSK
    pub fn update_entry(&mut self, entry: &AmsdosEntry) {
        // TODO refactor
        // compute the track/sector
        let (track, sector_id) = entry.track_and_sector(self.disc, self.head);

        // eprintln!("head:{:?} track: {} sector: {}", &self.head, track, sector_id);
        let mut sector = self
            .disc
            .sector_read_bytes(self.head, track, sector_id)
            .unwrap();
        let idx_in_sector: usize = ((entry.idx & 15) << 5) as usize;
        let bytes = &mut (sector[idx_in_sector..(idx_in_sector + AmsdosEntry::len())]);
        bytes.copy_from_slice(entry.as_bytes().as_ref());
        self.disc
            .sector_write_bytes(self.head, track, sector_id, &sector)
            .unwrap();
    }

    /// Rewrite the whole catalog
    pub fn set_catalog(&mut self, entries: &AmsdosEntries) {
        assert_eq!(64, entries.entries.len());
        for entry in &entries.entries {
            self.update_entry(entry);
        }
    }

    /// Add the given amsdos file to the disc
    /// Code is greatly inspired by idsk with no special verifications.
    /// In case of error, the disk is in a broken state => part of the file may be stored...
    pub fn add_file(
        &mut self,
        file: &AmsdosFile,
        is_system: bool,
        is_read_only: bool,
        behavior: AmsdosAddBehavior
    ) -> Result<(), AmsdosError> {
        // handle the case where a file is already present
        let filename = file.amsdos_filename()?;
        if let Some(_file) = self.get_file(filename) {
            match behavior {
                AmsdosAddBehavior::FailIfPresent => {
                    return Err(AmsdosError::FileAlreadyExists(format!("{:?}", filename)));
                },
                AmsdosAddBehavior::ReplaceIfPresent => {
                    self.erase_file(filename, false)?;
                },
                AmsdosAddBehavior::BackupIfPresent => {
                    let mut backup_fname = filename.clone();
                    backup_fname.set_extension("BAK");

                    if self.entries_for(backup_fname).is_some() {
                        self.erase_file(backup_fname, true)?;
                    }
                    assert!(self.get_file(backup_fname).is_none());
                    self.rename(filename, backup_fname)?;
                    assert!(self.get_file(filename).is_none());
                    assert!(self.get_file(backup_fname).is_some());
                },
                AmsdosAddBehavior::ReplaceAndEraseIfPresent => {
                    self.erase_file(filename, true)?;
                }
            }
        }

        // we know there is no file of the same name and can safely add it
        let content: Vec<u8> = file.header_and_content().copied().collect();

        let mut file_pos = 0;
        let file_size = content.len();
        let mut nb_entries = 0;

        let mut available_blocs = self.catalog().available_blocs();

        // println!("File size {} bytes", file_size);
        while file_pos < file_size {
            //     println!("File pos {}", file_pos);
            let entry_idx = match self.catalog().one_empty_entry() {
                Some(entry) => entry.idx,
                None => return Err(AmsdosError::NoEntriesAvailable)
            };

            // eprintln!("Will use entry  {entry_idx}");
            //      println!("Select entry {}", entry_idx);
            let entry_num_page = nb_entries;
            nb_entries += 1;

            // TODO need to find and comment why there is such strange maths
            let page_size = {
                let mut size = (file_size - file_pos + 127) >> 7;
                if size > 128 {
                    size = 128;
                }
                size
            };

            // //Nombre de blocs=TaillePage/8 arrondi par le haut
            let nb_blocs = (page_size + 7) >> 3;
            let chosen_blocs = {
                let mut chosen_blocs = [BlocIdx::default(); 16];
                for current_chosen_bloc in chosen_blocs.iter_mut().take(nb_blocs) {
                    let bloc_idx = match available_blocs.next() {
                        Some(bloc_idx) => bloc_idx,
                        None => return Err(AmsdosError::NoBlocAvailable)
                    };
                    assert!(bloc_idx.is_valid());
                    *current_chosen_bloc = bloc_idx;
                    // eprintln!("Will use bloc {:?}", bloc_idx);
                    //          println!("Select bloc{:?}", bloc_idx);
                    self.update_bloc(
                        bloc_idx,
                        &AmsdosManagerNonMut::<'dsk, D>::padding(
                            &content[file_pos..(file_pos + 2 * DATA_SECTOR_SIZE).min(file_size)],
                            2 * DATA_SECTOR_SIZE
                        )
                    )?;
                    // eprintln!("Has updated  bloc {:?}", bloc_idx);

                    file_pos += 2 * DATA_SECTOR_SIZE;
                }
                chosen_blocs
            };

            // Update the entry on disc
            let new_entry = AmsdosEntry {
                idx: entry_idx,
                file_name: file.amsdos_filename()?,
                read_only: is_read_only,
                system: is_system,
                num_page: entry_num_page,
                page_size: page_size as u8,
                blocs: chosen_blocs
            };
            self.update_entry(&new_entry)
        }
        Ok(())
    }

    /// Write bloc content on disc. One bloc use 2 sectors
    /// Implementation is stolen to iDSK
    pub fn update_bloc(&mut self, bloc_idx: BlocIdx, content: &[u8]) -> Result<(), String> {
        assert!(bloc_idx.is_valid());

        // More tests are needed to check if it can work without that
        assert_eq!(content.len(), DATA_SECTOR_SIZE * 2);

        let access_info = self.bloc_access_information(bloc_idx);

        // Copy in first sector
        self.disc.sector_write_bytes(
            self.head,
            access_info.track1,
            access_info.sector1_id,
            &content[0..DATA_SECTOR_SIZE]
        )?;

        // Copy in second sector
        self.disc.sector_write_bytes(
            self.head,
            access_info.track2,
            access_info.sector2_id,
            &content[DATA_SECTOR_SIZE..2 * DATA_SECTOR_SIZE]
        )?;
        Ok(())
    }

    fn erase_bloc(&mut self, bloc_idx: BlocIdx) -> Result<(), String> {
        assert!(bloc_idx.is_valid());

        let content = vec![0xE5; DATA_SECTOR_SIZE * 2];
        self.update_bloc(bloc_idx, &content)
    }

    pub fn erase_file(
        &mut self,
        filename: AmsdosFileName,
        clear_sectors: bool
    ) -> Result<(), AmsdosError> {
        let entries = self
            .entries_for(filename)
            .ok_or_else(|| AmsdosError::FileDoesNotExist(format!("{:?}", filename)))?;

        if clear_sectors {
            entries.iter().flat_map(|e| e.used_blocs()).for_each(|b| {
                self.erase_bloc(*b).unwrap();
            });
        }

        entries.iter().for_each(|e| {
            self.erase_entry(e);
        });

        Ok(())
    }

    pub fn rename(
        &mut self,
        source: AmsdosFileName,
        destination: AmsdosFileName
    ) -> Result<(), AmsdosError> {
        match self.entries_for(source) {
            Some(entries) => {
                entries.into_iter().for_each(|mut e| {
                    e.file_name = destination;
                    self.update_entry(&e);
                });
                Ok(())
            },
            None => Err(AmsdosError::FileDoesNotExist(format!("{:?}", source)))
        }
    }

    pub fn catalog<'mngr: 'dsk>(&'dsk self) -> AmsdosEntries {
        let nonmut: AmsdosManagerNonMut<'dsk, D> = self.into();
        nonmut.catalog()
    }

    fn bloc_access_information<'mngr: 'dsk>(
        &'mngr self,
        bloc_idx: BlocIdx
    ) -> BlocAccessInformation {
        let nonmut: AmsdosManagerNonMut<'dsk, D> = self.into();
        nonmut.bloc_access_information(bloc_idx)
    }

    pub fn get_file<'mngr: 'dsk, F: Into<AmsdosFileName>>(
        &'mngr self,
        filename: F
    ) -> Option<AmsdosFile> {
        let nonmut: AmsdosManagerNonMut<'dsk, D> = self.into();
        nonmut.get_file(filename)
    }

    fn entries_for<'mngr: 'dsk, F: Into<AmsdosFileName>>(
        &'mngr self,
        filename: F
    ) -> Option<Vec<AmsdosEntry>> {
        let nonmut: AmsdosManagerNonMut<'dsk, D> = self.into();
        nonmut.entries_for(filename)
    }
}

impl<'dsk, 'mngr: 'dsk, D: Disc> Into<AmsdosManagerNonMut<'dsk, D>>
    for &'mngr AmsdosManagerMut<'dsk, D>
{
    fn into(self) -> AmsdosManagerNonMut<'dsk, D> {
        AmsdosManagerNonMut {
            disc: self.disc,
            head: self.head
        }
    }
}

#[derive(Debug)]
pub struct AmsdosManagerNonMut<'dsk, D: Disc> {
    disc: &'dsk D,
    head: Head
}

#[allow(missing_docs)]
impl<'dsk, 'mng: 'dsk, D: Disc> AmsdosManagerNonMut<'dsk, D> {
    pub fn dsk(&'mng self) -> &'dsk D {
        self.disc
    }

    pub fn new_from_disc<S: Into<Head>>(disc: &'dsk D, head: S) -> Self {
        Self {
            disc,
            head: head.into()
        }
    }

    /// Format the disc. Currently it only modifies the catalog
    pub fn format(&mut self) {
        let _catalog = self.catalog();
        unimplemented!();
    }

    /// Return the entries of the Amsdos catalog
    /// Panic if dsk is not compatible
    pub fn catalog(&self) -> AmsdosEntries {
        let mut entries = Vec::new();
        let bytes = self
            .disc
            .consecutive_sectors_read_bytes(self.head, 0, DATA_FIRST_SECTOR_NUMBER, 4)
            .unwrap();

        for idx in 0..DIRECTORY_SIZE
        // (bytes.len() / 32)
        {
            let entry_buffer = &bytes[(idx * 32)..(idx + 1) * 32];
            let entry = AmsdosEntry::from_buffer(idx as u8, array_ref!(entry_buffer, 0, 32));
            entries.push(entry);
        }

        AmsdosEntries { entries }
    }

    /// Print the catalog on screen
    pub fn print_catalog(&self) {
        let entries = self.catalog();
        for entry in entries.visible_entries() {
            if !entry.is_erased() && !entry.is_system() {
                println!("{}", entry.format());
            }
        }
    }

    /// Retrieve the AmsdosEntry dedidacted to the specific file
    fn entries_for<F: Into<AmsdosFileName>>(&self, filename: F) -> Option<Vec<AmsdosEntry>> {
        let filename = filename.into();
        let entries = self
            .catalog()
            .for_file(&filename)
            .map(Clone::clone)
            .collect::<Vec<_>>();
        if entries.is_empty() {
            None
        }
        else {
            Some(entries)
        }
    }

    fn read_entries(&self, entries: &[AmsdosEntry]) -> Vec<u8> {
        entries
            .iter()
            .flat_map(|entry| self.read_entry(entry))
            .collect::<Vec<u8>>()
    }

    /// Return the file if it exists
    pub fn get_file<F: Into<AmsdosFileName>>(&self, filename: F) -> Option<AmsdosFile> {
        // Collect the entries for the given file
        let entries = self.entries_for(filename)?;

        //     println!("{:?}", &entries);

        // Retreive the binary data
        let content = self.read_entries(&entries);
        let mut file = AmsdosFile::from_buffer(&content);
        file.shrink_content_to_fit_header_size();

        Some(file)
    }

    /// Returns a Vec<u8> of the right size by padding 0
    pub fn padding(data: &[u8], size: usize) -> Vec<u8> {
        if data.len() == size {
            data.to_vec()
        }
        else if data.len() > size {
            unreachable!()
        }
        else {
            let _missing = size - data.len();
            let mut data = data.to_vec();
            data.resize(size, 0);
            data
        }
    }

    /// Returns the appropriate information to access the bloc
    /// Blindly stolen to iDSK
    fn bloc_access_information(&self, bloc_idx: BlocIdx) -> BlocAccessInformation {
        assert!(bloc_idx.is_valid());

        // Compute the information to access the first sector
        let sector_pos = bloc_idx.sector();
        let min_sector = self.disc.track_min_sector(self.head, 0);
        let track = {
            let mut track = bloc_idx.track();
            if min_sector == 0x41 {
                track += 2; // we skip file table ?
            }
            else if min_sector == 0x01 {
                track += 1;
            }
            track
        };

        if track > self.disc.nb_tracks_per_head() - 1 {
            unimplemented!(
                "Need to format track. [{:?}] => {} > {}",
                bloc_idx,
                track,
                self.disc.nb_tracks_per_head() - 1
            );
        }

        let track1 = track;
        let sector1_id = sector_pos + min_sector;

        // Compute the information to access the second sector
        // TODO set this knowledge in edsk
        let (sector2_id, track2) = {
            if (sector_pos + 1) > 8 {
                (0 + min_sector, track + 1)
            }
            else {
                (sector_pos + 1 + min_sector, track)
            }
        };

        BlocAccessInformation {
            track1,
            sector1_id,
            track2,
            sector2_id
        }
    }

    /// Read the content of the given bloc
    pub fn read_bloc(&self, bloc_idx: BlocIdx) -> Vec<u8> {
        assert!(bloc_idx.is_valid());
        let access_info = self.bloc_access_information(bloc_idx);

        let sector1_data = self
            .disc
            .sector_read_bytes(self.head, access_info.track1, access_info.sector1_id)
            .unwrap();

        let sector2_data = self
            .disc
            .sector_read_bytes(self.head, access_info.track2, access_info.sector2_id)
            .unwrap();

        let mut content = sector1_data;
        content.extend(sector2_data);

        assert_eq!(content.len(), DATA_SECTOR_SIZE * 2);

        content
    }

    /// Read the content of the given entry
    pub fn read_entry(&self, entry: &AmsdosEntry) -> Vec<u8> {
        entry
            .used_blocs()
            .iter()
            .flat_map(|bloc_idx| self.read_bloc(*bloc_idx))
            .collect::<Vec<u8>>()
    }
}

/// http://www.cpcwiki.eu/index.php/AMSDOS_Header
#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub struct AmsdosHeader {
    content: [u8; 128]
}

impl std::fmt::Debug for AmsdosHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "File: {:?}", self.amsdos_filename())?;
        writeln!(f, "Type: {:?}", self.file_type())?;
        writeln!(f, "Size 0x{:x}", self.file_length())?;
        writeln!(f, "Loading address 0x{:x}", self.loading_address())?;
        writeln!(f, "Execution address 0x{:x}", self.execution_address())?;
        writeln!(f, "Checksum 0x{:x}", self.checksum())?;
        Ok(())
    }
}

impl PartialEq for AmsdosHeader {
    fn eq(&self, other: &Self) -> bool {
        self.content[..68] == other.content[..68]
    }
}

#[allow(missing_docs)]
impl AmsdosHeader {
    /// Generate a header for binary file
    pub fn compute_binary_header(
        filename: &AmsdosFileName,
        loading_address: u16,
        execution_address: u16,
        data: &[u8]
    ) -> AmsdosHeader {
        AmsdosHeader::build_header(
            filename,
            AmsdosFileType::Binary,
            loading_address,
            execution_address,
            data.len() as u16
        )
    }

    /// Generate a header for a basic file
    pub fn compute_basic_header(filename: &AmsdosFileName, data: &[u8]) -> AmsdosHeader {
        AmsdosHeader::build_header(
            filename,
            AmsdosFileType::Basic,
            0x0170,
            0x0000,
            data.len() as _
        )
    }

    /// XXX currently untested
    pub fn build_header(
        filename: &AmsdosFileName,
        file_type: AmsdosFileType,
        loading_address: u16,
        execution_address: u16,
        data_len: u16
    ) -> Self {
        let mut content = Self { content: [0; 128] };

        content.set_amsdos_filename(filename);
        content.set_file_type(file_type);
        content.set_loading_address(loading_address);
        content.set_file_length(data_len);
        content.set_execution_address(execution_address);
        content.update_checksum();

        content
    }

    pub fn from_buffer(buffer: &[u8]) -> Self {
        Self {
            content: *array_ref!(buffer, 0, 128)
        }
    }

    pub fn set_user(&mut self, user: u8) -> &mut Self {
        self.content[0] = user;
        self
    }

    pub fn user(&self) -> u8 {
        self.content[0]
    }

    pub fn set_amsdos_filename(&mut self, filename: &AmsdosFileName) -> &mut Self {
        self.set_user(filename.user());
        self.set_filename(filename.filename_header_format());
        self.set_extension(filename.extension_header_format());
        self
    }

    /// Return the filename if possible
    pub fn amsdos_filename(&self) -> Result<AmsdosFileName, AmsdosError> {
        AmsdosFileName::new_incorrect_case(self.user(), &self.filename(), &self.extension())
    }

    pub fn set_filename(&mut self, filename: &[u8; 8]) -> &mut Self {
        for (idx, val) in filename.iter().enumerate() {
            self.content[1 + idx] = *val;
        }
        self
    }

    pub fn filename(&self) -> String {
        self.content[1..9]
            .iter()
            .filter_map(|&c| {
                if c == b' ' {
                    None
                }
                else {
                    Some(c as char)
                }
            })
            .collect::<String>()
    }

    pub fn set_extension(&mut self, extension: &[u8; 3]) -> &mut Self {
        for (idx, val) in extension.iter().enumerate() {
            self.content[9 + idx] = *val;
        }
        self
    }

    pub fn extension(&self) -> String {
        self.content[9..(9 + 3)]
            .iter()
            .filter_map(|&c| {
                if c == b' ' {
                    None
                }
                else {
                    Some(c as char)
                }
            })
            .collect::<String>()
    }

    pub fn set_file_type(&mut self, file_type: AmsdosFileType) -> &mut Self {
        self.content[18] = file_type as u8;
        self
    }

    pub fn file_type(&self) -> Result<AmsdosFileType, AmsdosError> {
        self.content[18].try_into()
    }

    pub fn set_loading_address(&mut self, address: u16) -> &mut Self {
        self.set_16bits_value(address, 21)
    }

    pub fn loading_address(&self) -> u16 {
        self.get_16bits_value(21)
    }

    pub fn set_file_length(&mut self, length: u16) -> &mut Self {
        self.set_16bits_value(length, 24);
        self.content[64] = self.content[24];
        self.content[65] = self.content[25];
        self.content[66] = 0;
        self
    }

    pub fn file_length(&self) -> u16 {
        self.get_16bits_value(24)
    }

    pub fn file_length2(&self) -> u16 {
        self.get_16bits_value(64)
    }

    pub fn set_execution_address(&mut self, execution_address: u16) -> &mut Self {
        self.set_16bits_value(execution_address, 26)
    }

    pub fn execution_address(&self) -> u16 {
        self.get_16bits_value(26)
    }

    #[allow(clippy::identity_op)]
    fn set_16bits_value(&mut self, value: u16, at: usize) -> &mut Self {
        self.content[at + 0] = (value % 256) as u8;
        self.content[at + 1] = (value / 256) as u8;
        self
    }

    #[allow(clippy::identity_op)]
    fn get_16bits_value(&self, at: usize) -> u16 {
        let low = u16::from(self.content[at + 0]);
        let high = u16::from(self.content[at + 1]);

        256 * high + low
    }

    pub fn checksum(&self) -> u16 {
        self.get_16bits_value(67)
    }

    pub fn update_checksum(&mut self) -> &mut Self {
        let checksum = self.compute_checksum();
        self.set_16bits_value(checksum, 67);
        self
    }

    pub fn compute_checksum(&self) -> u16 {
        self.content[0..=66]
            .iter()
            .map(|&b| u16::from(b))
            .sum::<u16>()
    }

    /// Checks if the stored checksum correspond to the expected checksum
    pub fn is_checksum_valid(&self) -> bool {
        self.checksum() == self.compute_checksum()
    }

    pub fn as_bytes(&self) -> &[u8; 128] {
        &self.content
    }
}

/// Encode an amsdos file.
#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct AmsdosFile {
    header: AmsdosHeader,
    content: Vec<u8>
}

#[allow(missing_docs)]
impl AmsdosFile {
    /// Create a binary file and add build the header accordingly to the arguments
    pub fn binary_file_from_buffer(
        filename: &AmsdosFileName,
        loading_address: u16,
        execution_address: u16,
        data: Vec<u8>
    ) -> Result<Self, AmsdosError> {
        if data.len() > 0x10000 {
            return Err(AmsdosError::FileLargerThan64Kb);
        }

        let header = AmsdosHeader::build_header(
            filename,
            AmsdosFileType::Binary,
            loading_address,
            execution_address,
            data.len() as _
        );

        Ok(Self {
            header,
            content: data
        })
    }

    pub fn basic_file_from_buffer(
        filename: &AmsdosFileName,
        data: Vec<u8>
    ) -> Result<Self, AmsdosError> {
        if data.len() > 0x10000
        // TODO shorten the limit
        {
            return Err(AmsdosError::FileLargerThan64Kb);
        }

        let header = AmsdosHeader::build_header(
            filename,
            AmsdosFileType::Basic,
            0x0170,
            0x0000,
            data.len() as _
        );

        Ok(Self {
            header,
            content: data
        })
    }

    /// Create a file from its header and content
    pub fn from_buffer(data: &[u8]) -> Self {
        let (header_bytes, content_bytes) = data.split_at(128);
        Self {
            header: AmsdosHeader::from_buffer(header_bytes),
            content: content_bytes.to_vec()
        }
    }

    /// Read a file from disc and success if there is no io error and if the header if correct
    pub fn open_valid<P: AsRef<Path>>(path: P) -> Result<Self, AmsdosError> {
        let mut f = File::open(path.as_ref())?;
        let mut content = Vec::new();
        f.read_to_end(&mut content)?;

        if content.len() < 128 {
            return Err(AmsdosError::InvalidHeader);
        }

        let ams_file = Self::from_buffer(&content);
        if ams_file.header().is_checksum_valid() {
            Ok(ams_file)
        }
        else {
            Err(AmsdosError::InvalidHeader)
        }
    }

    pub fn amsdos_filename(&self) -> Result<AmsdosFileName, AmsdosError> {
        self.header.amsdos_filename()
    }

    /// Return an iterator on the full content of the file: header + content
    pub fn header_and_content(&self) -> impl Iterator<Item = &u8> {
        self.header.as_bytes().iter().chain(self.content.iter())
    }

    pub fn header(&self) -> &AmsdosHeader {
        &self.header
    }

    pub fn content(&self) -> &[u8] {
        self.content.as_ref()
    }

    /// Returns the header + the content
    pub fn as_bytes(&self) -> Vec<u8> {
        self.header_and_content().copied().collect()
    }

    /// Files are read from disc by chunks of the size of 2 sectors.
    /// This method removes the extra unecessary bytes.
    pub fn shrink_content_to_fit_header_size(&mut self) {
        let size = self.header.file_length() as usize;
        assert!(
            size <= self.content.len(),
            "Header is invalid. File content has a size of {}, whereas header specify {}",
            self.content.len(),
            size
        );
        self.content.resize(size, 0);
    }

    /// Save the file at the given path
    pub fn save_in_folder<P: AsRef<Path>>(&self, folder: P) -> std::io::Result<()> {
        use std::io::Write;

        let folder = folder.as_ref();
        let fname = self.amsdos_filename().unwrap().filename();
        println!("Will write in {}", fname);
        let mut file = File::create(folder.join(fname))?;
        file.write_all(&self.as_bytes())?;
        Ok(())
    }
}
