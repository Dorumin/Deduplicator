A file deduplication tool that runs in a folder of your choosing

Are there other tools that do this same thing? Probably.

Would I have saved time by using them instead of making my own? Definitely.

Did I have more fun and get to play around with thread pools, file streaming, and hashing? Well, try for yourself and tell me about it

```powershell
$ deduplicator --help
deduplicator 0.1.0
Deduplicates files in a folder

USAGE:
    deduplicator.exe [FLAGS] [OPTIONS] --path <path>

FLAGS:
        --delete          Whether to delete the duplicate files
    -h, --help            Prints help information
        --no-recursive    Whether to not search subfolders recursively
    -V, --version         Prints version information

OPTIONS:
        --keep <keep>          What file to keep; `first` or `last` [default: first]
        --order <order>        How to order files; `modified`, `created`, `name` [default: modified]
        --path <path>          Path towards the folder to scan
        --threads <threads>    How many threads to split file reading into [default: 4]
```
