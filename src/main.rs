// Helper to get absolute path even if file doesn't exist
fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir().unwrap().join(path)
    }
}
mod database;
mod cli;

use std::{env, error::Error, fmt::{self, Debug, Display, Formatter}, io, os::unix::fs, path::{Path, PathBuf}};
use clap::{Parser, ValueEnum};
use std::fs::read_link;
use serde::{Deserialize, Serialize};

use crate::database::LinkStorage;
use crate::cli::{Cli, Commands};

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let db = LinkStorage::init(&env::current_dir().unwrap());

    match cli.command {
        Commands::Create { source, target, link_type } => {
            let abs_source = absolute_path(&source);
            let abs_target = absolute_path(&target);
            let already_exists = db.get_quicklink(abs_source.to_str().unwrap(), abs_target.to_str().unwrap()).is_some();
            if already_exists {
                eprintln!("A link for source '{}' and target '{}' already exists in the database.", abs_source.display(), abs_target.display());
                return Ok(());
            }
            let quicklink = QuickLink::new(&source, &target, link_type);
            match quicklink {
                Ok(mut link) => {
                    link.link()?;
                    db.save_quicklink(&link);
                    println!("Link created: {}", link);
                },
                Err(e) => {
                    eprintln!("Error creating link: {}", e);
                }
            }
        }
        Commands::Remove { target } => {
            match db.find_by_target(&target) {
                Some(mut link) => {
                    if link.exists {
                        link.unlink()?;
                        println!("Link removed: {}", link);
                    } else {
                        println!("Link not present in filesystem: {}", link);
                    }
                },
                None => {
                    eprintln!("No tracked link found for target: {}", target.display());
                }
            }
        }
        Commands::List => {
            let links = db.get_all();
            println!("Tracked links:");
            for link in links {
                println!("{}", link);
            }
        }
        Commands::Toggle { target } => {
            match db.find_by_target(&target) {
                Some(mut link) => {
                    link.toggle_link()?;
                    db.save_quicklink(&link);
                    println!("Toggled link: {}", link);
                },
                None => {
                    eprintln!("No tracked link found for target: {}", target.display());
                }
            }
        }
    }
    Ok(())
}


enum QuickLinkCreationError {
    /// Format: source
    SourceDoesNotExist(String), 
    /// Format: source, target
    TargetExists(String, String),
    /// Format: source, target, target's source
    TargetLinkHasDifferentSource(String, String, String), 
    /// Format: source, linktype, targettype
    UnavailableLinkType(String, LinkType, FileType), 
    /// Format: io_error
    LinkIOError(io::Error)
}

impl Error for QuickLinkCreationError {}

impl Display for QuickLinkCreationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            QuickLinkCreationError::SourceDoesNotExist(source_path) => write!(f, "Link for {} cannot be created - source does not exist", source_path),
            QuickLinkCreationError::TargetExists(source_path, target_path) => write!(f, "Link for {} cannot be created - target ({}) exists", source_path, target_path),
            QuickLinkCreationError::TargetLinkHasDifferentSource(source_path, target_path, different_source) => write!(f, "Link for {} cannot be created - target ({}) is already a link from {}", source_path, target_path, different_source),
            QuickLinkCreationError::UnavailableLinkType(source_path, linktype, targettype) => write!(f, "Link for {} cannot be created - link type {} is incompatible with source type: {}", source_path, linktype, targettype),
            QuickLinkCreationError::LinkIOError(ioerror) => write!(f, "Encountered an io error while linking: {}", ioerror),
        }
    }
}

impl Debug for QuickLinkCreationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            QuickLinkCreationError::SourceDoesNotExist(source_path) => write!(f, "Link for {} cannot be created - source does not exist", source_path),
            QuickLinkCreationError::TargetExists(source_path, target_path) => write!(f, "Link for {} cannot be created - target ({}) exists", source_path, target_path),
            QuickLinkCreationError::TargetLinkHasDifferentSource(source_path, target_path, different_source) => write!(f, "Link for {} cannot be created - target ({}) is already a link from {}", source_path, target_path, different_source),
            QuickLinkCreationError::UnavailableLinkType(source_path, linktype, targettype) => write!(f, "Link for {} cannot be created - link type {} is incompatible with source type: {}", source_path, linktype, targettype),
            QuickLinkCreationError::LinkIOError(ioerror) => write!(f, "Encountered an io error while linking: {}", ioerror),
        }
    }
}

impl From<std::io::Error> for QuickLinkCreationError {
    fn from(value: std::io::Error) -> Self {
        QuickLinkCreationError::LinkIOError(value)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Deserialize, Default)]
enum LinkType {
    #[default]
    Softlink,
    Hardlink
}

impl Display for LinkType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LinkType::Softlink =>  write!(f, "Softlink"),
            LinkType::Hardlink =>  write!(f, "Hardlink"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum FileType {
    File,
    Directory,
}

impl Display for FileType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            FileType::File => write!(f, "File"),
            FileType::Directory => write!(f, "Directory"),
        }
    }
}


#[derive(Serialize, Deserialize)]
/// A soft/hard link wrapper, that remembers what it is.
/// Can be not present in the filesystem.
struct QuickLink {
    source: PathBuf,
    target: PathBuf,
    exists: bool,
    linktype: LinkType,
}

impl QuickLink {
    /// Create a new QuickLink object, without linking it.
    /// Supports importing an existing softlink, provided the target file is already one pointing exactly to the source.
    pub fn new(source: &Path, target: &Path, linktype: LinkType) -> Result<QuickLink, QuickLinkCreationError> {
        let abs_source = absolute_path(source);
        let abs_target = absolute_path(target);
        if !abs_source.exists() {
            return Err(QuickLinkCreationError::SourceDoesNotExist(abs_source.to_string_lossy().into_owned()));
        }
        let mut exists = false;
        if abs_target.exists() {
            exists = true;
            if linktype == LinkType::Softlink && abs_target.is_symlink() {
                if read_link(&abs_target).unwrap().as_path() != abs_source.canonicalize().unwrap() {
                    return Err(QuickLinkCreationError::TargetLinkHasDifferentSource(abs_source.to_string_lossy().into_owned(), abs_target.to_string_lossy().into_owned(), read_link(&abs_target).unwrap().as_path().to_string_lossy().to_string()))
                }
            }
            else if linktype == LinkType::Hardlink {
                // There might be a way to do it later, for now - always abort, as if it was just a file.
            }
            else {
                return Err(QuickLinkCreationError::TargetExists(abs_source.to_string_lossy().into_owned(), abs_target.to_string_lossy().into_owned()));
            }
        }
        if abs_target.is_dir() && (linktype == LinkType::Hardlink) {
            return Err(QuickLinkCreationError::UnavailableLinkType(abs_source.to_string_lossy().into_owned(), linktype, FileType::Directory));
        }
        Ok(QuickLink { source: abs_source, target: abs_target, exists, linktype })
    }

    /// Create a new QuickLink object, without linking it.
    /// Supports importing an existing softlink, provided the target file is already one pointing exactly to the source.
    pub fn new_autolink(source: &Path, target: &Path, linktype: LinkType) -> Result<QuickLink, QuickLinkCreationError> {
        let mut link = QuickLink::new(source, target, linktype)?;
        if !link.exists {
            link.link()?;
        }
        Ok(link)
    }


    pub fn toggle_link(&mut self) -> std::io::Result<()> {
        match self.exists {
            true => self.unlink()?,
            false => self.link()?,
        }
        Ok(())
    }

    pub fn link(&mut self) -> std::io::Result<()> {

        match self.linktype {
            LinkType::Softlink => self.softlink(),
            LinkType::Hardlink => self.hardlink(),
        }?;
        self.exists = true;
        Ok(())
    }

    fn softlink(&self) -> std::io::Result<()>{
        fs::symlink(&self.source, &self.target)?;
        Ok(())
    }

    fn hardlink(&self) -> std::io::Result<()>{
        std::fs::hard_link(&self.source, &self.target)?;
        Ok(())
    }

    pub fn unlink(&mut self) -> std::io::Result<()> {
        std::fs::remove_file(&self.target)?; // links to directories are still just files
        self.exists = false;
        Ok(())
    }
}

impl Display for QuickLink {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} -> {} , e: {}, t: {}", self.source.to_str().unwrap(), self.target.to_str().unwrap(), self.exists, self.linktype)
    }
}

