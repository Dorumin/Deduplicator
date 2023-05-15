#![deny(clippy::all)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

mod options;
mod deduplicator;
mod similarity;

use clap::Parser;
use deduplicator::Deduplicator;

use options::{Options, Mode};
use similarity::Similarity;

fn main() {
    ctrlc::set_handler(|| {
        eprint!("{}", ansi_escapes::CursorShow);
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let options = Options::parse();

    match options.mode {
        Mode::Hash => {
            let deduplicator = Deduplicator::new(options);

            deduplicator.execute();
        },
        Mode::Similarity => {
            let similarity = Similarity::new(options);

            similarity.execute();
        }
    }
}
