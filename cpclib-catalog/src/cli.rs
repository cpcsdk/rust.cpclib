
use clap::{Parser, Subcommand, value_parser};
use cpclib_catart::{Locale, interpret::Mode};
use log::error;

#[derive(Parser, Debug)]
#[command(name = "catalog")]
#[command(about = "Amsdos catalog manipulation tool.", author = "Krusty/Benediction")]
pub struct CatalogApp {
    /// Input file that contains the entries of the catalog (a binary file or a dsk). For 'build' command, this is the BASIC file if not specified in the command.
    pub input_file: Option<String>,
    
    #[command(subcommand)]
    pub command: CatalogCommand,
}

/// Shared rendering options for PNG output and locale selection
#[derive(Parser, Debug)]
pub struct RenderOptions {
    /// Optional PNG file to save pixel-accurate rendering of the catart
    #[arg(long = "png")]
    pub png_output: Option<String>,
    
    /// Font locale to use when generating PNG (english, french, spanish, german, danish). Defaults to english.
    #[arg(long = "locale", default_value = "english", alias="language")]
    pub locale: String,
    
    /// Screen mode to use for catart rendering (0, 1, 2, or 3). Defaults to mode 1.
    #[arg(long = "mode", default_value = "1")]
    pub mode: u8,
}

impl RenderOptions {
    /// Parse the locale string into a Locale enum, with error handling
    pub fn parse_locale(&self) -> Locale {
        match self.locale.to_lowercase().as_str() {
            "english" | "en" => Locale::English,
            "french" | "fr" => Locale::French,
            "spanish" | "es" => Locale::Spanish,
            "german" | "de" => Locale::German,
            "danish" | "da" => Locale::Danish,
            _ => {
                error!("Unknown locale '{}', defaulting to English. Valid options: english, french, spanish, german, danish", self.locale);
                Locale::English
            }
        }
    }
    
    /// Parse the mode value into a Mode enum, with validation
    pub fn parse_mode(&self) -> Mode {
        match self.mode {
            0 => Mode::Mode0,
            1 => Mode::Mode1,
            2 => Mode::Mode2,
            _ => {
                error!("Invalid mode '{}', defaulting to Mode 1. Valid options: 0, 1, 2", self.mode);
                Mode::Mode1
            }
        }
    }
    
    /// Get PNG output path as Option<&str>
    pub fn png_path(&self) -> Option<&str> {
        self.png_output.as_deref()
    }
}

#[derive(Subcommand, Debug)]
pub enum CatalogCommand {
    /// Display the catalog using CatArt rendering (sorted alphabetically)
    Cat {
        #[command(flatten)]
        render_options: RenderOptions,
    },
    
    /// Display the catalog using CatArt rendering (directory order, unsorted)
    Dir {
        #[command(flatten)]
        render_options: RenderOptions,
    },
    
    /// List the content of the catalog ONLY for files having no control chars
    List,
    
    /// List the content of the catalog EVEN for files having control chars
    Listall,
    
    /// Build a catart from a BASIC program. Output will be a DSK/HFE file if the output filename ends with .dsk or .hfe, otherwise a raw 2048-byte catalog binary.
    Build {
        /// BASIC file to convert to catart (optional if input_file is provided at top level)
        basic_file: Option<String>,
        
        /// Output file (defaults to catart.dsk). Use .dsk or .hfe extension for disc images, otherwise creates raw binary
        #[arg(short = 'o', long = "output")]
        output_file: Option<String>,
        
        #[command(flatten)]
        render_options: RenderOptions,
    },

    /// Extract the Basic listing from the input dsk. If no --output is provided the listing is printed on standard output otherwhise it is saved in the provided filname
    Decode {
        /// Optional output file for the decoded BASIC listing. If not provided, prints to stdout.
        #[arg(short = 'o', long = "output")]
        output_file: Option<String>,
    },
    
    /// Modify an entry in the catalog
    Modify {
        /// Selects the entry to modify
        #[arg(long, value_parser = value_parser!(u8).range(..=63))]
        entry: u8,
        
        /// Set the selected entry readonly
        #[arg(long = "readonly")]
        setreadonly: bool,
        
        /// Set the selected entry hidden
        #[arg(long = "system")]
        setsystem: bool,
        
        /// Set the selected entry read and write
        #[arg(long = "noreadonly")]
        unsetreadonly: bool,
        
        /// Set the selected entry visible
        #[arg(long = "nosystem")]
        unsetsystem: bool,
        
        /// Set the user value
        #[arg(long)]
        user: Option<u8>,
        
        /// Set the filename of the entry
        #[arg(long)]
        filename: Option<String>,
        
        /// Set the blocs to load (and update the number of blocs accordingly to that)
        #[arg(long, num_args = ..=16)]
        blocs: Option<Vec<u8>>,
        
        /// Set the page number
        #[arg(long)]
        numpage: Option<String>,
        
        /// Force the size of the entry
        #[arg(long)]
        size: Option<String>,
    },
    
    /// Debug catart by displaying each entry's bytes and corresponding BASIC commands
    Debug {
        /// Display entries in catalog (sorted alphabetically) order
        #[arg(long)]
        cat: bool,
        
        /// Display entries in directory (unsorted) order
        #[arg(long)]
        dir: bool,
    },
}