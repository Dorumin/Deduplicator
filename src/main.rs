#![deny(clippy::all)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

mod options;

use std::io;
use std::fs;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use ring::digest::{SHA256, Digest, Context};
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};
use threadpool::ThreadPool;

use options::Options;

use crate::options::OutputSort;

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
    start: Instant,
    options: Options,
    pool: ThreadPool,
    sizes: HashMap<u64, Vec<DirEntry>>
}

impl Deduplicator {
    fn new(options: Options) -> Self  {
        Self {
            start: Instant::now(),
            pool: ThreadPool::new(options.threads.unwrap_or_else(|| num_cpus::get())),
            options,
            sizes: HashMap::new()
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
            .filter_map(Result::ok)
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
            .filter_map(|(meta, entry)| {
                if let Ok(metadata) = meta {
                    Some((metadata, entry))
                } else {
                    None
                }
            })
    }

    fn select<'dirs>(&self, files: &'dirs [DirEntry]) -> (&'dirs DirEntry, Vec<&'dirs DirEntry>) {
        let mut mapped: Vec<_> = Self::map_with_metadata(files).collect();

        match self.options.order.as_str() {
            "modified" => {
                mapped.sort_by_cached_key(|(meta, _)| meta.modified().unwrap());
            },
            "created" => {
                mapped.sort_by_cached_key(|(meta, _)| meta.created().unwrap());
            },
            "name" => {
                mapped.sort_by_cached_key(|(_, entry)| entry.file_name());
            },
            _ => unreachable!()
        }

        let mut sorted: Vec<_> = mapped.into_iter()
            .map(|(_, entry)| entry)
            .collect();

        match self.options.keep.as_ref() {
            "first" => {
                let first = sorted.remove(0);

                (first, sorted)
            },
            "last" => {
                let last = sorted.pop().unwrap();

                (last, sorted)
            },
            _ => unreachable!()
        }
    }

    fn execute(mut self) {
        self.collect();

        self.consume();
    }

    fn collect(&mut self) {
        let (tx, rx) = mpsc::channel();
        let mut iterations = 0;

        let entries: Vec<_> = self.list_entries().collect();
        let count = entries.len();

        println!("Found {} files", count);

        self.sizes.reserve(count);

        for entry in entries {
            iterations += 1;

            let tx = tx.clone();
            self.pool.execute(move || {
                let metadata = match entry.metadata() {
                    Err(_) => {
                        tx.send(None).expect("channel is available for sending");
                        return;
                    },
                    Ok(v) => v
                };

                tx.send(Some((metadata, entry))).expect("channel is available for sending");
            });
        }

        eprint!("{}", ansi_escapes::CursorHide);

        for (idx, (metadata, entry)) in rx.iter().take(iterations).filter_map(|x| x).enumerate() {
            eprint!("\rProcessed {} files out of {}", idx, count);

            self.sizes.entry(metadata.len())
                .or_insert_with(Vec::new)
                .push(entry);
        }

        eprintln!("{}", ansi_escapes::CursorShow);
        println!();
    }

    fn shorten_path(&self, path: &Path) -> String {
        let path_char_count = self.options.path.to_string_lossy().chars().count();

        path.to_string_lossy()
            .chars()
            // Skip the path characters + 1 for the leading path separator
            .skip(path_char_count + 1)
            .collect()
    }

    fn get_true_dupes(entries: &[DirEntry]) -> (Vec<Vec<&DirEntry>>, i32) {
        if entries.len() == 1 {
            return (Vec::new(), 0);
        }

        let mut map: HashMap<Vec<u8>, Vec<&DirEntry>> = HashMap::new();

        for entry in entries {
            let digest = match Self::digest(entry) {
                None => continue,
                Some(digest) => digest
            };

            map.entry(digest)
                .or_insert_with(Vec::new)
                .push(entry);
        }

        let mut dupes = Vec::new();
        let mut collisions = 0;

        for (_, entries) in map.into_iter() {
            if entries.len() > 1 {
                dupes.push(entries);
            } else {
                collisions += 1;
            }
        }

        (dupes, collisions)
    }

    fn consume(mut self) {
        let mut duplicate_groups = 0;
        let mut duplicate_count = 0;
        let mut collision_count = 0;

        let elapsed = self.start.elapsed();

        let mut files: Vec<_> = self.sizes.into_values().collect();
        self.sizes = HashMap::new();

        if let Some(ref sorter) = self.options.sort_output {
            match sorter {
                OutputSort::Created => {
                    files.sort_by_cached_key(|f| f.last().unwrap().metadata().unwrap().created().unwrap())
                }
                OutputSort::Modified => {
                    files.sort_by_cached_key(|f| f.last().unwrap().metadata().unwrap().modified().unwrap())
                },
                OutputSort::Name => {
                    files.sort_by(|a, b| {
                        let a_name = a.last().unwrap().file_name();
                        let b_name = b.last().unwrap().file_name();

                        a_name.cmp(b_name)
                    });
                }
            }
        }

        for files in files {
            let (dupes_vec, collisions) = Self::get_true_dupes(&files);

            collision_count += collisions;

            for dupes in dupes_vec {
                let cloned: Vec<_> = dupes.into_iter().cloned().collect();
                let (source, duplicates) = self.select(&cloned);

                if !self.options.quiet {
                    println!("Found {} duplicate files:", duplicates.len() + 1);
                    println!("Source: {}", self.shorten_path(source.path()));

                    for file in &duplicates {
                        let short_path = self.shorten_path(file.path());

                        println!("Copy:   {}", short_path);
                    }
                }

                if self.options.delete {
                    Self::delete(&duplicates);
                }

                if !self.options.quiet {
                    println!();
                }

                duplicate_groups += 1;
                duplicate_count += duplicates.len() + 1;
            }
        }

        println!("Summary:");
        println!("{} duplicate groups", duplicate_groups);
        println!("{} duplicates found", duplicate_count);
        println!("{} size collisions", collision_count);
        println!();
        println!("Done in {}ms!", self.start.elapsed().as_millis());
        println!("Scan took {}ms", elapsed.as_millis());
    }

    fn delete(duplicates: &[&DirEntry]) {
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
    ctrlc::set_handler(|| {
        eprint!("{}", ansi_escapes::CursorShow);
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let options = Options::from_args();
    let deduplicator = Deduplicator::new(options);

    deduplicator.execute();
}
