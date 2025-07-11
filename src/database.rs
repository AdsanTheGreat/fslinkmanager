use core::panic;
use std::{fs::{create_dir, File, OpenOptions}, io::{BufReader, BufWriter, Write}, path::{Path, PathBuf}};
use blake2::{Blake2b512, Digest};


use crate::QuickLink;

pub struct LinkStorage {
    folder_path: PathBuf,
    link_folder: PathBuf
}

impl LinkStorage {
    pub fn new(initial_path: &PathBuf) -> LinkStorage {
        let folder_path: PathBuf;
        let mut current_searched_path = initial_path.canonicalize().unwrap(); // Make the path absolute
        'search: loop {
            for e in current_searched_path.read_dir().expect("Failed to read initial search directory") {
                let entry = e.unwrap();
                if entry.file_name() == ".fslink" {
                    folder_path = entry.path();
                    break 'search;
                }
            }
            if current_searched_path.as_os_str() == "/" { // Search reached the root directory
                panic!("No error handling in linkstorage yet! - search reached the root directory");
                break 'search;
            }
            current_searched_path = current_searched_path.parent().unwrap().to_path_buf();
        }
        let link_folder = folder_path.join("links");
        if !dir_contains(&folder_path, "links") {
            create_dir(folder_path.join("links")).unwrap();
        }
        
        //println!("{} {}", folder_path.display(), link_folder.display());
        LinkStorage { folder_path, link_folder }
    }

    /// Get a QuickLink by its source and target path (using hash as filename)
    pub fn get_quicklink(&self, source: &str, target: &str) -> Option<QuickLink> {
        let hash = hash_source_target(source, target);
        let file_path = self.link_folder.join(hash);
        if file_path.exists() {
            let target_file_reader = BufReader::new(File::open(file_path).unwrap());
            let resolved_link: QuickLink = serde_json::from_reader(target_file_reader).unwrap();
            return Some(resolved_link);
        }
        None
    }

    /// Get all saved QuickLinks as a Vec
    pub fn get_all(&self) -> Vec<QuickLink> {
        let mut links = Vec::new();
        if let Ok(entries) = self.link_folder.read_dir() {
            for entry in entries.flatten() {
                if let Ok(file) = File::open(entry.path()) {
                    if let Ok(link) = serde_json::from_reader::<_, QuickLink>(BufReader::new(file)) {
                        links.push(link);
                    }
                }
            }
        }
        links
    }

    /// Save a QuickLink to a file named by a hash of its source and target path
    pub fn save_quicklink(&self, link: &QuickLink) {
        let source_str = link.source.to_string_lossy();
        let target_str = link.target.to_string_lossy();
        let hash = hash_source_target(&source_str, &target_str);
        let target_file = OpenOptions::new().read(true).write(true).truncate(true).create(true)
                        .open(self.link_folder.join(hash)).unwrap();
        let mut target_file_writer = BufWriter::new(target_file);
        let serialized = serde_json::to_string(link).unwrap();
        target_file_writer.write(serialized.as_bytes()).unwrap();
    }

    pub fn init(initial_path: &PathBuf) -> LinkStorage {
        if !dir_contains(&initial_path, ".fslink") {
            create_dir(initial_path.join(".fslink")).unwrap();
        }
        LinkStorage::new(initial_path)

    }
}

fn dir_contains(directory: &PathBuf, target_name: &str) -> bool {
    for e in directory.read_dir().expect("Failed to read initial search directory") {
        let entry = e.unwrap();
        if entry.file_name() == target_name {
            return true;
            }
        }
    false
}

/// Hash source and target path to a hex string using Blake2b
fn hash_source_target(source: &str, target: &str) -> String {
    let mut hasher = Blake2b512::new();
    hasher.update(source.as_bytes());
    hasher.update(b"|");
    hasher.update(target.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes for brevity
}