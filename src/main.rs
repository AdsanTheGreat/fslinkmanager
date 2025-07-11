mod database;

use std::{env, error::Error, fmt::{self, Debug, Display, Formatter}, io, os::unix::fs, path::{Path, PathBuf}, str::FromStr};
use std::fs::read_link;
use serde::{Deserialize, Serialize};

use crate::database::{LinkStorage};

fn main() -> std::io::Result<()> {
    let db = LinkStorage::init(&env::current_dir().unwrap().join(Path::new("tmp")));
    let mut link: QuickLink = QuickLink::new(Path::new("./tmp/file1"), Path::new("./tmp/link1"), LinkType::Softlink).unwrap();
    let mut link2: QuickLink = QuickLink::new_autolink(Path::new("./tmp/file2"), Path::new("./tmp/link2"), LinkType::Softlink).unwrap();
    link.toggle_link()?;
    link2.toggle_link()?;
    println!("{}", link);
    println!("{}", link2);
    db.save_quicklink(&link);
    db.save_quicklink(&link2);

    let file1_canon = Path::new("./tmp/file1").canonicalize().unwrap();
    let link1_path = Path::new("./tmp/link1").to_path_buf();
    println!("loaded: {}", db.get_quicklink(&file1_canon.to_string_lossy(), &link1_path.to_string_lossy()).unwrap());

    println!("All saved links:");
    for l in db.get_all() {
        println!("{}", l);
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

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
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
        if !source.exists() {
            return Err(QuickLinkCreationError::SourceDoesNotExist(source.to_string_lossy().into_owned()));
        }
        let mut exists = false;
        if target.exists() { // Check if it is a link, if it has correct type, abort if it is
            exists = true;
            if linktype == LinkType::Softlink && target.is_symlink() {
                if read_link(target).unwrap().as_path() != source.canonicalize().unwrap() { // softlink exists, but source is different.
                    return Err(QuickLinkCreationError::TargetLinkHasDifferentSource(source.to_string_lossy().into_owned(), target.to_string_lossy().into_owned(), read_link(target).unwrap().as_path().to_string_lossy().to_string()))
                }
            }
            else if linktype == LinkType::Hardlink {
                // There might be a way to do it later, for now - always abort, as if it was just a file.
            }
            else {
                return Err(QuickLinkCreationError::TargetExists(source.to_string_lossy().into_owned(), target.to_string_lossy().into_owned())); // target is just a file/directory
            }
        }
        if Path::new(target).is_dir() && (linktype == LinkType::Hardlink) {
            return Err(QuickLinkCreationError::UnavailableLinkType(source.to_string_lossy().into_owned(), linktype, FileType::Directory));
        }
        Ok(QuickLink { source: source.to_path_buf().canonicalize().unwrap(), target: target.to_path_buf(), exists: exists, linktype: linktype })
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

