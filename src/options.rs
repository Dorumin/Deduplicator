use structopt::StructOpt;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, StructOpt)]
#[structopt(name = "deduplicator", about = "Deduplicates files in a folder")]
pub struct Options {
    #[structopt(long, parse(from_os_str), help = "Path towards the folder to scan")]
    pub path: PathBuf,

    #[structopt(long, default_value = "first", help = "What file to keep; `first` or `last`")]
    pub keep: String,

    // TODO: Make an enum
    #[structopt(long, default_value = "modified", help = "How to order files; `modified`, `created`, `name`")]
    pub order: String,

    #[structopt(long, help = "Whether to delete the duplicate files")]
    pub delete: bool,

    #[structopt(long, help = "Whether to shut the fuck up")]
    pub quiet: bool,

    #[structopt(long, help = "How many threads to split file reading into")]
    pub threads: Option<usize>,

    #[structopt(long, help = "Whether to not search subfolders recursively")]
    pub no_recursive: bool,

    #[structopt(long, help = "Whether to show the summary at the end")]
    pub no_summary: bool,

    #[structopt(long, help = "How to sort the duplicate groups; `modified`, `created`, `name`")]
    pub sort_output: Option<OutputSort>
}

#[derive(Debug)]
pub enum OutputSort {
    Modified,
    Created,
    Name
}

#[derive(Debug)]
pub struct OutputSortErr(String);

impl FromStr for OutputSort {
    type Err = OutputSortErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "modified" => Ok(Self::Modified),
            "created" => Ok(Self::Created),
            "name" => Ok(Self::Name),
            _ => Err(OutputSortErr(s.to_string()))
        }
    }
}

impl Display for OutputSortErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Expected one of `modified`, `created`, or `name`; found {}", self.0))
    }
}
