# rindexer

A simple file indexer in rust to be able to find duplicates and search files on large & slow volumes (ie. NAS)

## Build

Run `cargo build --release` or `make`, binary will be in `target/release/rindexer`.

## Cross compile for ARM7 systems / Synology

Run `make arm` or `make`, binary will be in `target/armv7-unknown-linux-musleabihf/release/rindexer`.

## Author

Benjamin Gentil <benjamin@gentil.io>

## Copyright

MIT