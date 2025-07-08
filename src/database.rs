use std::{fs::{create_dir, File, OpenOptions}, io::{BufWriter, Write}, path::{Path, PathBuf}};

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
                todo!("No error handling in linkstorage yet! - search reached the root directory");
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

    pub fn get_quicklink(&self, target_name: &str) -> Option<QuickLink> {
        for e in self.link_folder.read_dir().expect("Failed to read initial search directory") {
                let entry = e.unwrap();
                if entry.file_name() == target_name {
                    let target_file_reader = File::open_buffered(entry.path()).unwrap();
                    let resolved_link: QuickLink = serde_json::from_reader(target_file_reader).unwrap();
                    return Some(resolved_link);
                }
            }
        None
    }

    pub fn save_quicklink(&self, link: &QuickLink) {
        let target_string = link.source.file_name().unwrap().to_str().unwrap();
        let target_file;
        //println!("{}", self.link_folder.join(target_string).display());
        target_file = OpenOptions::new().read(true).write(true).truncate(true).create(true)
                        .open(self.link_folder.join(target_string)).unwrap();
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