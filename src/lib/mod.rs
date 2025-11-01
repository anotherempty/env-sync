//! Environment file synchronization library.
//!
//! This library provides functionality to parse `.env` files while preserving comments
//! and synchronize them with template files. It's designed for scenarios where you
//! want to maintain a template structure while preserving local customizations.
//!
//! # Features
//!
//! - **Zero-copy parsing**: Uses `Cow<str>` for efficient string handling
//! - **Comment preservation**: Maintains both preceding and inline comments
//! - **Flexible synchronization**: Merges template structure with local values
//! - **Optional tracing**: Detailed logging when the `tracing` feature is enabled
//!
//! # Example
//!
//! ```rust,no_run
//! use env_sync::sync::{EnvSync, EnvSyncOptions};
//! use std::path::PathBuf;
//!
//! let options = EnvSyncOptions {
//!     local_file: None, // defaults to .env
//!     template_file: PathBuf::from(".env.template"),
//! };
//!
//! EnvSync::sync_with_options(options).unwrap();
//! ```

pub mod parse;
pub mod sync;
