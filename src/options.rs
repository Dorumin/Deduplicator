use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[clap(name = "deduplicator", about = "Deduplicates files in a folder")]
pub struct Options {
    #[clap(long, help = "Path towards the folder to scan")]
    pub path: PathBuf,

    #[clap(long, value_enum, default_value = "first", help = "What file to keep; `first` or `last`")]
    pub keep: Keep,

    // // TODO: Make an enum
    // #[clap(long, default_value = "modified", help = "How to order files; `modified`, `created`, `name`")]
    // pub order: String,

    #[clap(long, value_enum, default_value = "modified", help = "How to order files; `modified`, `created`, `name`")]
    pub order: FileOrdering,

    #[clap(long, help = "Whether to delete the duplicate files")]
    pub delete: bool,

    #[clap(long, help = "Whether to shut the fuck up")]
    pub quiet: bool,

    #[clap(long, default_value_t = num_cpus::get(), help = "How many threads to split file reading into")]
    pub threads: usize,

    #[clap(long, help = "Whether to not search subfolders recursively")]
    pub no_recursive: bool,

    #[clap(long, help = "Whether to show the summary at the end")]
    pub no_summary: bool,

    #[clap(long, help = "Whether to not ignore errors (e.g. retrieving and reading files)")]
    pub no_ignore_errors: bool,

    #[clap(long, value_enum, help = "How to sort the duplicate groups; `modified`, `created`, `name`")]
    pub sort_output: Option<FileOrdering>,

    #[clap(long, value_enum, default_value = "hash", help = "Criteria for file duplicate finding; `hash` or `similarity`")]
    pub mode: Mode,

    #[clap(long, default_value = "95", help = "Required similarity for reporting duplicate images. Used in similarity mode. 0-100, 100 indicating exact match")]
    pub similarity_score: u32
}

#[derive(ValueEnum, Debug, Clone)]
pub enum FileOrdering {
    Modified,
    Created,
    Name
}

#[derive(ValueEnum, Debug, Clone)]
pub enum Keep {
    First,
    Last
}

#[derive(ValueEnum, Debug, Clone)]
pub enum Mode {
    Hash,
    Similarity
}
