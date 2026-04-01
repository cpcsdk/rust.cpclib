use std::fs::File;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::path::Path;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::clap::{self, Arg, ArgAction, Command, CommandFactory, FromArgMatches, Parser, Subcommand};
use cpclib_runner::event::EventObserver;
#[allow(unused_imports)]
use cpclib_runner::runner::{Runner, RunnerWithClap};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Archive as TarArchive;

use crate::task::ARCHIVE_CMDS;

#[derive(Parser, Debug)]
#[command(name = "archive", about = "Create, list, and extract archives (.zip or .tar.gz)", arg_required_else_help = true)]
struct ArchiveArgs {
    #[command(subcommand)]
    command: Option<ArchiveCommand>
}

#[derive(Subcommand, Debug)]
enum ArchiveCommand {
    /// Create a new archive
    #[command(disable_help_flag = false)]
    Create {
        /// Archive file to create (.zip or .tar.gz)
        #[arg(short, long)]
        output: String,

        /// Files or directories to add to the archive
        #[arg(required = true)]
        files: Vec<String>
    },

    /// List contents of an archive
    List {
        /// Archive file to list
        archive: String
    },

    /// Extract contents of an archive
    Extract {
        /// Archive file to extract
        archive: String,

        /// Destination directory (defaults to current directory)
        #[arg(short, long)]
        dest: Option<String>
    }
}

#[derive(Debug, Clone, Copy)]
enum ArchiveFormat {
    Zip,
    TarGz
}

impl ArchiveFormat {
    fn from_path(path: &str) -> Result<Self, String> {
        let path_lower = path.to_lowercase();
        if path_lower.ends_with(".zip") {
            Ok(ArchiveFormat::Zip)
        }
        else if path_lower.ends_with(".tar.gz") || path_lower.ends_with(".tgz") {
            Ok(ArchiveFormat::TarGz)
        }
        else {
            Err(format!(
                "Unsupported archive format for '{}'. Use .zip or .tar.gz",
                path
            ))
        }
    }
}

pub struct ArchiveRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for ArchiveRunner<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: EventObserver> ArchiveRunner<E> {
    pub fn new() -> Self {
        Self {
            command: ArchiveArgs::command()
                .no_binary_name(true),
            _phantom: PhantomData::<E>
        }
    }
}

impl<E: EventObserver> RunnerWithClap for ArchiveRunner<E> {
    fn get_clap_command(&self) -> &Command {
        &self.command
    }

    fn get_matches<S: AsRef<str>>(
        &self,
        itr: &[S],
        e: &dyn EventObserver
    ) -> Result<Option<clap::ArgMatches>, String> {
        match self
            .get_clap_command()
            .clone()
            .try_get_matches_from(itr.iter().map(|s| s.as_ref()))
        {
            Ok(args) => Ok(Some(args)),
            Err(err) => {
                use clap::error::ErrorKind;
                match err.kind() {
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                        e.emit_stdout(&err.to_string());
                        Ok(None)
                    },
                    _ => {
                        e.emit_stderr(&err.to_string());
                        Err(String::from("Argument parsing failed"))
                    }
                }
            }
        }
    }
}

impl<E: EventObserver> Runner for ArchiveRunner<E> {
    type EventObserver = E;

    fn get_command(&self) -> &str {
        ARCHIVE_CMDS[0]
    }

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let Some(matches) = self.get_matches(itr, o)?
        else {
            return Ok(());
        };
        let args = ArchiveArgs::from_arg_matches(&matches).map_err(|e| e.to_string())?;

        let Some(command) = args.command
        else {
            return Ok(());
        };

        match command {
            ArchiveCommand::Create { output, files } => {
                create_archive(&output, &files, o)?;
            },
            ArchiveCommand::List { archive } => {
                list_archive(&archive, o)?;
            },
            ArchiveCommand::Extract { archive, dest } => {
                extract_archive(&archive, dest.as_deref(), o)?;
            }
        }

        Ok(())
    }
}

fn create_archive<E: EventObserver>(
    output: &str,
    files: &[String],
    o: &E
) -> Result<(), String> {
    let format = ArchiveFormat::from_path(output)?;

    match format {
        ArchiveFormat::Zip => create_zip(output, files, o),
        ArchiveFormat::TarGz => create_tar_gz(output, files, o)
    }
}

fn create_zip<E: EventObserver>(output: &str, files: &[String], o: &E) -> Result<(), String> {
    let file = File::create(output).map_err(|e| format!("Failed to create {}: {}", output, e))?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for file_path in files {
        let path = Utf8Path::new(file_path);
        if path.is_file() {
            add_file_to_zip(&mut zip, path, options, o)?;
        }
        else if path.is_dir() {
            add_dir_to_zip(&mut zip, path, path, options, o)?;
        }
        else {
            return Err(format!("Path not found: {}", file_path));
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    o.emit_stdout(&format!("Created archive: {}", output));
    Ok(())
}

fn add_file_to_zip<W: Write + std::io::Seek, E: EventObserver>(
    zip: &mut zip::ZipWriter<W>,
    path: &Utf8Path,
    options: zip::write::SimpleFileOptions,
    o: &E
) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path, e))?;
    let name = path.as_str();

    zip.start_file(name, options)
        .map_err(|e| format!("Failed to add {} to zip: {}", name, e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;

    zip.write_all(&buffer)
        .map_err(|e| format!("Failed to write {} to zip: {}", name, e))?;

    o.emit_stdout(&format!("  Added: {}", name));
    Ok(())
}

fn add_dir_to_zip<W: Write + std::io::Seek, E: EventObserver>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Utf8Path,
    base: &Utf8Path,
    options: zip::write::SimpleFileOptions,
    o: &E
) -> Result<(), String> {
    for entry in
        fs_err::read_dir(dir).map_err(|e| format!("Failed to read directory {}: {}", dir, e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = Utf8PathBuf::try_from(entry.path())
            .map_err(|e| format!("Invalid UTF-8 path: {}", e))?;

        if path.is_file() {
            add_file_to_zip(zip, &path, options, o)?;
        }
        else if path.is_dir() {
            add_dir_to_zip(zip, &path, base, options, o)?;
        }
    }

    Ok(())
}

fn create_tar_gz<E: EventObserver>(output: &str, files: &[String], o: &E) -> Result<(), String> {
    let tar_gz = File::create(output).map_err(|e| format!("Failed to create {}: {}", output, e))?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);

    for file_path in files {
        let path = Utf8Path::new(file_path);
        if path.is_file() {
            tar.append_path(path.as_str())
                .map_err(|e| format!("Failed to add {} to tar: {}", file_path, e))?;
            o.emit_stdout(&format!("  Added: {}", file_path));
        }
        else if path.is_dir() {
            tar.append_dir_all(path.as_str(), path.as_str())
                .map_err(|e| format!("Failed to add directory {} to tar: {}", file_path, e))?;
            o.emit_stdout(&format!("  Added directory: {}", file_path));
        }
        else {
            return Err(format!("Path not found: {}", file_path));
        }
    }

    tar.finish()
        .map_err(|e| format!("Failed to finalize tar.gz: {}", e))?;

    o.emit_stdout(&format!("Created archive: {}", output));
    Ok(())
}

fn list_archive<E: EventObserver>(archive: &str, o: &E) -> Result<(), String> {
    let format = ArchiveFormat::from_path(archive)?;

    match format {
        ArchiveFormat::Zip => list_zip(archive, o),
        ArchiveFormat::TarGz => list_tar_gz(archive, o)
    }
}

fn list_zip<E: EventObserver>(archive: &str, o: &E) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("Failed to open {}: {}", archive, e))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive {}: {}", archive, e))?;

    o.emit_stdout(&format!("Contents of {}:", archive));
    for i in 0..zip.len() {
        let file = zip
            .by_index(i)
            .map_err(|e| format!("Failed to read entry {}: {}", i, e))?;
        o.emit_stdout(&format!(
            "  {} ({} bytes)",
            file.name(),
            file.size()
        ));
    }

    Ok(())
}

fn list_tar_gz<E: EventObserver>(archive: &str, o: &E) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("Failed to open {}: {}", archive, e))?;
    let dec = GzDecoder::new(file);
    let mut tar = TarArchive::new(dec);

    o.emit_stdout(&format!("Contents of {}:", archive));
    for entry in tar.entries().map_err(|e| format!("Failed to read tar: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to read entry path: {}", e))?;
        o.emit_stdout(&format!(
            "  {} ({} bytes)",
            path.display(),
            entry.size()
        ));
    }

    Ok(())
}

fn extract_archive<E: EventObserver>(
    archive: &str,
    dest: Option<&str>,
    o: &E
) -> Result<(), String> {
    let format = ArchiveFormat::from_path(archive)?;
    let dest_dir = dest.unwrap_or(".");

    match format {
        ArchiveFormat::Zip => extract_zip(archive, dest_dir, o),
        ArchiveFormat::TarGz => extract_tar_gz(archive, dest_dir, o)
    }
}

fn extract_zip<E: EventObserver>(archive: &str, dest: &str, o: &E) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("Failed to open {}: {}", archive, e))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive {}: {}", archive, e))?;

    fs_err::create_dir_all(dest)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;

    o.emit_stdout(&format!("Extracting {} to {}:", archive, dest));
    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .map_err(|e| format!("Failed to read entry {}: {}", i, e))?;
        let outpath = Path::new(dest).join(file.name());

        if file.name().ends_with('/') {
            fs_err::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        else {
            if let Some(parent) = outpath.parent() {
                fs_err::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
            o.emit_stdout(&format!("  Extracted: {}", file.name()));
        }
    }

    Ok(())
}

fn extract_tar_gz<E: EventObserver>(archive: &str, dest: &str, o: &E) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("Failed to open {}: {}", archive, e))?;
    let dec = GzDecoder::new(file);
    let mut tar = TarArchive::new(dec);

    fs_err::create_dir_all(dest)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;

    o.emit_stdout(&format!("Extracting {} to {}:", archive, dest));
    tar.unpack(dest)
        .map_err(|e| format!("Failed to extract tar.gz: {}", e))?;

    // List extracted files for feedback
    let file = File::open(archive).map_err(|e| format!("Failed to open {}: {}", archive, e))?;
    let dec = GzDecoder::new(file);
    let mut tar = TarArchive::new(dec);
    for entry in tar.entries().map_err(|e| format!("Failed to read tar: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to read entry path: {}", e))?;
        o.emit_stdout(&format!("  Extracted: {}", path.display()));
    }

    Ok(())
}
