# backup_dumper

## Description

A simple dumper for various backup formats. My goal is to document and better understand the different formats, NOT to be a complete restore tool.

## Usage

```
cargo run
```

## Currently Supported Formats
- Duplicacy
- Restic (only files, no folder structure)
- Knoxite (app must be modified to use JSON encoding instead of gob)
- BlobBackup (only files, no folder structure)