use std::{error::Error, fmt::{self, Debug, Display, Formatter}, io, os::unix::fs, path::Path};
use std::fs::read_link;
use serde::{Deserialize, Serialize};

fn main() -> std::io::Result<()> {
    let mut link: QuickLink = QuickLink::new("target", "target2", LinkType::Softlink).unwrap();
    link.toggle_link()?;
    
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
    source: String,
    target: String,
    exists: bool,
    linktype: LinkType,
}

impl QuickLink {
    /// Create a new QuickLink object, without linking it.
    /// Supports importing an existing softlink, provided the target file is already one pointing exactly to the source.
    pub fn new(source: &str, target: &str, linktype: LinkType) -> Result<QuickLink, QuickLinkCreationError> {
        let source_file = Path::new(source);
        if !source_file.exists() {
            return Err(QuickLinkCreationError::SourceDoesNotExist(source.to_owned()));
        }
        let target_file = Path::new(target);
        let mut exists = false;
        if target_file.exists() { // Check if it is a link, if it has correct type, abort if it is
            if linktype == LinkType::Softlink && target_file.is_symlink() {
                if read_link(target).unwrap().as_path() != source_file { // softlink exists, but source is different.
                    return Err(QuickLinkCreationError::TargetLinkHasDifferentSource(source.to_owned(), target.to_owned(), read_link(target).unwrap().as_path().to_string_lossy().to_string()))
                } else {
                    exists = true;
                }
            }
            else if linktype == LinkType::Hardlink {
                // There might be a way to do it later, for now - always abort, as if it was just a file.
            }
            else {
                return Err(QuickLinkCreationError::TargetExists(source.to_owned(), target.to_owned())); // target is just a file/directory
            }
        }
        if Path::new(target).is_dir() && (linktype == LinkType::Hardlink) {
            return Err(QuickLinkCreationError::UnavailableLinkType(source.to_owned(), linktype, FileType::Directory));
        }
        Ok(QuickLink { source: source.to_owned(), target: target.to_owned(), exists: exists, linktype: linktype })
    }
    
    /// Create a new QuickLink object, without linking it.
    /// Supports importing an existing softlink, provided the target file is already one pointing exactly to the source.
    pub fn new_autolink(source: &str, target: &str, linktype: LinkType) -> Result<QuickLink, QuickLinkCreationError> {
        let source_file = Path::new(source);
        if !source_file.exists() {
            return Err(QuickLinkCreationError::SourceDoesNotExist(source.to_owned()));
        }
        let target_file = Path::new(target);
        let mut exists = false;
        if target_file.exists() { // Check if it is a link, if it has correct type, abort if it is
            if linktype == LinkType::Softlink && target_file.is_symlink() {
                if read_link(target).unwrap().as_path() != source_file { // softlink exists, but source is different.
                    return Err(QuickLinkCreationError::TargetLinkHasDifferentSource(source.to_owned(), target.to_owned(), read_link(target).unwrap().as_path().to_string_lossy().to_string()))
                } else {
                    exists = true;
                }
            }
            else if linktype == LinkType::Hardlink {
                // There might be a way to do it later, for now - always abort, as if it was just a file.
                return Err(QuickLinkCreationError::TargetExists(source.to_owned(), target.to_owned())); // target is just a file/directory
            }
            else {
                return Err(QuickLinkCreationError::TargetExists(source.to_owned(), target.to_owned())); // target is just a file/directory
            }
        }
        if target_file.is_dir() && linktype == LinkType::Hardlink && !exists {
            return Err(QuickLinkCreationError::UnavailableLinkType(source.to_owned(), linktype, FileType::Directory));
        }
        let mut link = QuickLink { source: source.to_owned(), target: target.to_owned(), exists, linktype: linktype };
        if !exists {
            link.link()?;
        }
        Ok(link)
    }


    pub fn toggle_link(&mut self) -> std::io::Result<()> {
        match self.exists {
            true => self.unlink()?,
            false => self.link()?,
        }
        self.exists = !self.exists;
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

