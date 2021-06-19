// Require docs for public APIs, deny unsafe code, etc.
// #![forbid(unsafe_code, unused_must_use, unstable_features)]
// #![deny(
//     clippy::clone_on_ref_ptr,
//     trivial_casts,
//     trivial_numeric_casts,
//     missing_docs,
//     unreachable_pub,
//     unused_import_braces,
//     unused_extern_crates,
//     unused_qualifications
// )]

/// TODO: write docs
pub mod archive;
/// TODO: write docs
pub mod asset;
/// TODO: write docs
pub mod config;
/// Environments help group usable tools together, this can be helpful for workflows where you need to keep
/// mutlple versions of the same tool, for instance multiple different versions of `kubectl` to connect to
/// legacy clusters and new clusters
pub mod environment;
/// TODO: write docs
pub mod errors;
/// TODO: write docs
pub mod machine;
/// TODO: write docs
pub mod manager;
/// TODO: write docs
pub mod version;
