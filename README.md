# env-sync

[![Crates.io Version](https://img.shields.io/crates/v/env-sync)](https://crates.io/crates/env-sync) ![Crates.io Size (version)](https://img.shields.io/crates/size/env-sync/0.1.0) [![docs.rs](https://img.shields.io/docsrs/env-sync)](https://docs.rs/env-sync)

A Rust CLI tool and library for synchronizing an environment file with a template file while preserving local customizations and comments.

## Purpose

Keep your `.env` files in sync with a git-trackable `.env.template` file. The tool preserves your local values, comments, and customizations while adopting the structure from the template.

## Usage

```bash
# Sync .env with .env.template (default)
env-sync

# Specify custom files
env-sync -l .env.local -t .env.example

# Enable verbose logging
env-sync -v    # debug level
env-sync -vv   # trace level
```

## How it works

1. Uses the template file as the base structure
2. For each variable in the template:
   - If template value is empty but local has a value, keeps the local value
   - If template has no comments but local does, preserves local comments
3. Writes the result back to the local file

## Installation

```bash
# Using cargo
cargo install env-sync

# Using cargo-binstall
cargo binstall env-sync
```

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
