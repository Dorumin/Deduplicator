use std::io;
use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

use ring::digest::{SHA256, Digest, Context};
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};
use threadpool::ThreadPool;

#[derive(Debug, StructOpt)]
#[structopt(name = "deduplicator", about = "Deduplicates files in a folder")]
struct Options {
    #[structopt(long, parse(from_os_str), help = "Path towards the folder to scan")]
    path: PathBuf,

    #[structopt(long, default_value = "first", help = "What file to keep; `first` or `last`")]
    keep: String,

    #[structopt(long, default_value = "modified", help = "How to order files; `modified`, `created`, `name`")]
    order: String,

    #[structopt(long, help = "Whether to delete the duplicate files")]
    delete: bool,

    #[structopt(long, default_value = "4", help = "How many threads to split file reading into")]
    threads: usize,

    #[structopt(long, help = "Whether to not search subfolders recursively")]
    no_recursive: bool,
}

fn sha256_digest<R>(mut reader: R) -> io::Result<Digest>
where
    R: io::Read
{
    let mut ctx = Context::new(&SHA256);
    let mut buf = [0; 1024];

    loop {
        let count = reader.read(&mut buf)?;
        if count == 0 {
            break;
        }

        ctx.update(&buf[..count]);
    }

    Ok(ctx.finish())
}

struct Deduplicator {
    options: Options,
    pool: ThreadPool,
    map: HashMap<Vec<u8>, Vec<DirEntry>>
}

impl Deduplicator {
    fn new(options: Options) -> Self  {
        Deduplicator {
            pool: ThreadPool::new(options.threads),
            options,
            map: HashMap::new()
        }
    }

    fn list_entries(&self) -> impl Iterator<Item=DirEntry> {
        WalkDir::new(&self.options.path)
            .max_depth(if self.options.no_recursive {
                1
            } else {
                std::usize::MAX
            })
            .into_iter()
            .filter_map(|e| e.ok())
    }

    fn digest(entry: &DirEntry) -> Option<Vec<u8>> {
        let file = match fs::File::open(entry.path()) {
            // Ignore any inaccessible files or folders that can't be read
            Err(_) => {
                return None;
            },
            Ok(file) => file
        };
        let digest = match sha256_digest(io::BufReader::new(file)) {
            Err(_) => {
                return None;
            },
            Ok(digest) => digest
        };

        Some(digest.as_ref().to_owned())
    }

    fn map_with_metadata(files: &[DirEntry]) -> impl Iterator<Item=(fs::Metadata, &DirEntry)> {
        files.iter()
            .map(|entry| (entry.metadata(), entry))
            .filter(|(meta, _)| meta.is_ok())
            .map(|(meta, entry)| (meta.unwrap(), entry))
    }

    fn select<'dirs>(&self, files: &'dirs [DirEntry]) -> (&'dirs DirEntry, &'dirs [DirEntry]) {
        let mut mapped: Vec<_> = Deduplicator::map_with_metadata(files).collect();

        match self.options.order.as_str() {
            "modified" => {
                mapped.sort_by(|(a, _), (b, _)| {
                    a.modified().unwrap().cmp(&b.modified().unwrap())
                });
            },
            "created" => {
                mapped.sort_by(|(a, _), (b, _)| {
                    a.created().unwrap().cmp(&b.created().unwrap())
                });
            },
            "name" => {
                mapped.sort_by(|(_, a), (_, b)| {
                    a.file_name().cmp(&b.file_name())
                });
            },
            _ => unreachable!()
        }

        match self.options.keep.as_ref() {
            "first" => {
                let first = files.first().unwrap();

                (first, &files[1..])
            },
            "last" => {
                let last = files.last().unwrap();

                (last, &files[..files.len()])
            },
            _ => unreachable!()
        }
    }

    fn execute(&mut self) {
        self.collect();


        self.consume();
    }

    fn collect(&mut self) {
        let (tx, rx) = mpsc::channel();
        let mut iterations = 0;

        let entries: Vec<_> = self.list_entries().collect();
        let count = entries.len();

        println!("Found {} files", count);

        for entry in entries.into_iter() {
            iterations += 1;

            let tx = tx.clone();
            self.pool.execute(move || {
                let digest = match Deduplicator::digest(&entry) {
                    None => {
                        tx.send(None).expect("channel is available for sending");
                        return;
                    },
                    Some(digest) => digest
                };

                tx.send(Some((digest, entry))).expect("channel is available for sending");
            });
        }

        for (idx, (digest, entry)) in rx.iter().take(iterations).filter_map(|x| x).enumerate() {
            eprint!("\rProcessed {} files out of {}", idx, count);

            self.map.entry(digest)
                .or_insert_with(Vec::new)
                .push(entry);
        }
    }

    fn consume(&self) {
        let path_char_count = self.options.path.to_string_lossy().chars().count();

        for files in self.map.values() {
            if files.len() > 1 {
                let (source, duplicates) = self.select(files);

                println!("Found {} duplicate files:", files.len());
                println!("Source: {}", source.path().to_string_lossy());

                for file in duplicates.iter() {
                    let short_path: String = file.path().to_string_lossy()
                        .chars()
                        // Skip the path characters + 1 for the leading path separator
                        .skip(path_char_count + 1)
                        .collect();

                    println!("Copy: {}", short_path);
                }

                if self.options.delete {
                    self.delete(duplicates);
                }

                println!();
            }
        }
    }

    fn delete(&self, duplicates: &[DirEntry]) {
        for dup in duplicates.iter() {
            match fs::remove_file(dup.path()) {
                Ok(_) => {},
                Err(err) => {
                    eprintln!("Failure while deleting: {}", dup.path().to_string_lossy());
                    eprintln!("{:?}", err);
                    eprintln!();
                }
            }
        }
    }
}

fn main() {
    let options = Options::from_args();
    let mut deduplicator = Deduplicator::new(options);

    deduplicator.execute();
}
