A file deduplication tool that runs in a folder of your choosing

Are there other tools that do this same thing? Probably.

Would I have saved time by using them instead of making my own? Definitely.

Did I have more fun and get to play around with thread pools, file streaming, and hashing? Well, try for yourself and tell me about it

```powershell
$ deduplicator --help
deduplicator
Deduplicates files in a folder

USAGE:
    deduplicator.exe [OPTIONS] --path <PATH>

OPTIONS:
        --delete
            Whether to delete the duplicate files

    -h, --help
            Print help information

        --keep <KEEP>
            What file to keep; `first` or `last` [default: first] [possible values: first, last]

        --mode <MODE>
            Criteria for file duplicate finding; `hash` or `similarity` [default: hash] [possible
            values: hash, similarity]

        --no-ignore-errors
            Whether to not ignore errors (e.g. retrieving and reading files)

        --no-recursive
            Whether to not search subfolders recursively

        --no-summary
            Whether to show the summary at the end

        --order <ORDER>
            How to order files; `modified`, `created`, `name` [default: modified] [possible values:
            modified, created, name]

        --path <PATH>
            Path towards the folder to scan

        --quiet
            Whether to shut the fuck up

        --similarity-score <SIMILARITY_SCORE>
            Required similarity for reporting duplicate images. Used in similarity mode. 0-100, 100
            indicating exact match [default: 95]

        --sort-output <SORT_OUTPUT>
            How to sort the duplicate groups; `modified`, `created`, `name` [possible values:
            modified, created, name]

        --threads <THREADS>
            How many threads to split file reading into [default: 8]
```
