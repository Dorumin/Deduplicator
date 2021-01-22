use std::collections::HashMap;
use std::io;
use std::fs::File;
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

    #[structopt(long, default_value = "first", help = "What file to keep")]
    keep: String,

    #[structopt(long, default_value = "4", help = "How many threads to split file reading into")]
    threads: usize,

    #[structopt(long, help = "Whether to search subfolders recursively")]
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
        let file = match File::open(entry.path()) {
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

    fn execute(&mut self) {
        let (tx, rx) = mpsc::channel();
        let mut iterations = 0;

        for entry in self.list_entries() {
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

        for (digest, entry) in rx.iter().take(iterations).filter_map(|x| x) {
            self.map.entry(digest)
                .or_insert_with(Vec::new)
                .push(entry);
        }

        for files in self.map.values() {
            if files.len() > 1 {
                println!("Found {} duplicate files:", files.len());

                for file in files.iter() {
                    println!("{}", file.path().to_string_lossy());
                }

                println!();
            }
        }
    }
}

fn main() {
    let options = Options::from_args();
    let mut deduplicator = Deduplicator::new(options);

    deduplicator.execute();
}
