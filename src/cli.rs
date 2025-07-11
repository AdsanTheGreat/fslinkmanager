use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::LinkType;

/// CLI for fslinkmanager: manage filesystem links and track them in a local database.
#[derive(Parser)]
#[command(name = "fslinkmanager")]
#[command(about = "Manage filesystem links and track them locally", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new link
    Create {
        /// Source path
        /// Source file/directory (positional)
        source: PathBuf,
        /// Target path
        /// Target link path (positional)
        target: PathBuf,
        /// Link type, Softlink | Hardlink
        #[arg(value_enum)]
        link_type: LinkType,
    },
    /// Remove an existing link
    Remove {
        /// Target path
        /// Target link path (positional)
        target: PathBuf,
    },
    /// List all tracked links
    List,
    /// Toggle (enable/disable) a link
    Toggle {
        /// Target link path
        /// Target link path (positional)
        target: PathBuf,
    },
}

