use std::collections::{HashMap, HashSet};
use std::fs::Metadata;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use itertools::Itertools;
use threadpool::ThreadPool;
use image_hasher::{ImageHash, HasherConfig, HashAlg};
use walkdir::{DirEntry, WalkDir};

use crate::options::Options;

pub struct Similarity {
    start: Instant,
    options: Options,
    pool: ThreadPool,
    hashes: Vec<(ImageHash, Metadata, DirEntry)>
}

impl Similarity {
    pub fn new(options: Options) -> Self {
        Self {
            start: Instant::now(),
            pool: ThreadPool::new(options.threads),
            hashes: Vec::new(),
            options,
        }
    }

    pub fn execute(mut self) {
        self.consume();

        eprintln!("File consumption took {}ms", self.start.elapsed().as_millis());

        self.collect();

        eprintln!("Finished! Took {}ms", self.start.elapsed().as_millis());
    }

    fn list_entries(&self) -> impl Iterator<Item=DirEntry> {
        let no_ignore_errors = self.options.no_ignore_errors;

        WalkDir::new(&self.options.path)
            .max_depth(if self.options.no_recursive {
                1
            } else {
                std::usize::MAX
            })
            .into_iter()
            .inspect(move |result| {
                if let Err(err) = result {
                    if no_ignore_errors {
                        eprintln!("Found error while walking directory: {err:?}");
                    }
                }
            })
            .filter_map(Result::ok)
    }

    fn consume(&mut self) {
        let (tx, rx) = mpsc::channel();
        let mut iterations = 0;

        let entries: Vec<_> = self.list_entries().collect();
        let count = entries.len();

        eprintln!("Found {} files", count);

        self.hashes.reserve(count);

        for entry in entries {
            iterations += 1;

            let tx = tx.clone();
            let no_ignore_errors = self.options.no_ignore_errors;
            self.pool.execute(move || {

                let metadata = match entry.metadata() {
                    Err(_) => {
                        tx.send(None).expect("channel is available for sending");
                        return;
                    },
                    Ok(v) => v
                };

                if !metadata.is_file() {
                    tx.send(None).expect("channel is available for sending");
                    return;
                }

                let path = entry.path();
                let image = image::open(path);

                let image = match image {
                    Err(err) => {
                        if no_ignore_errors {
                            eprintln!("Could not read file as image in similarity mode:");
                            eprintln!("{err:?}");
                            eprintln!("{path:?}");
                        }

                        tx.send(None).expect("channel is available for sending");
                        return;
                    },
                    Ok(image) => image
                };

                let hash_config = HasherConfig::new()
                    .hash_alg(HashAlg::DoubleGradient)
                    .hash_size(16, 16)
                    .to_hasher();
                let hash = hash_config.hash_image(&image);

                tx.send(Some((hash, metadata, entry))).expect("channel is available for sending");
            });
        }

        eprint!("{}", ansi_escapes::CursorHide);

        for (idx, (hash, metadata, entry)) in rx.iter().take(iterations).filter_map(|x| x).enumerate() {
            eprint!("\rProcessed {} files out of {}", idx, count);

            self.hashes.push((hash, metadata, entry));
        }

        eprintln!("{}", ansi_escapes::CursorShow);
        eprintln!();
    }

    fn collect(&self) {
        let start_collect = Instant::now();
        let combinations = self.hashes.iter().tuple_combinations();
        let required_similarity = (self.options.similarity_score as f32) / 100.0;

        let mut duplicate_pairs = Vec::new();

        for (a, b) in combinations {

            let (hasha, _, filea) = a;
            let (hashb, _, fileb) = b;

            let max_dist = hasha.as_bytes().len() * 8;
            let dist = hasha.dist(hashb);

            let dist = if dist == 0 {
                0.0
            } else {
                (dist as f32) / (max_dist as f32)
            };
            let similarity_score = 1.0 - dist;

            if similarity_score < required_similarity {
                continue;
            }

            duplicate_pairs.push((similarity_score, filea, fileb));
        }

        // Collect all duplicate pairs into *duplicate groups*
        // Any file that's recognized as a duplicate gets mapped into a single group
        // This does NOT compare complex similarity scores between each file;
        // if a compares similar to b, and b compares similar to c,
        // a, b, and c will be in the same group. Even though a may not be similar to c
        // I'm honestly not sure of a foolproof way to solve this, although
        // this shouldn't be an issue if a high similarity threshold is chosen
        // Malicious input files may interfere if there are many very-similar files
        // slowly in a gradient towards a different file
        // TODO: Could special case this for similarity = 100%
        let mut duplicate_group_indices: HashMap<&Path, usize> = HashMap::new();
        let mut duplicate_groups = Vec::new();

        for (similarity_score, filea, fileb) in duplicate_pairs {
            let mut group_index = None;
            if group_index.is_none() && duplicate_group_indices.contains_key(filea.path()) {
                group_index = duplicate_group_indices.get(filea.path()).map(|n| *n);
            }
            if group_index.is_none() && duplicate_group_indices.contains_key(fileb.path()) {
                group_index = duplicate_group_indices.get(fileb.path()).map(|n| *n);
            }
            if group_index.is_none() {
                group_index = Some(duplicate_groups.len());
                duplicate_groups.push(SimilarityGroup {
                    similarity_score,
                    set: HashSet::new()
                });
            }

            let group_index = group_index.unwrap();
            let group = &mut duplicate_groups[group_index];

            group.set.insert(filea.path());
            group.set.insert(fileb.path());

            duplicate_group_indices.insert(filea.path(), group_index);
            duplicate_group_indices.insert(fileb.path(), group_index);
        }

        eprintln!("Collection done! Took {}ms", start_collect.elapsed().as_millis());

        for group in duplicate_groups {
            print!("{} ", group.similarity_score);

            for file_path in group.set.iter() {
                print!("{:?} ", file_path);
            }

            println!();
        }
    }
}

pub struct SimilarityGroup<'a> {
    similarity_score: f32,
    set: HashSet<&'a Path>
}
