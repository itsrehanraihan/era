//! ERA project model: `era.toml`, project discovery, asset paths and the
//! `era new` template. Contains no language parsing (that arrives in M2).

pub mod assets;
pub mod discover;
pub mod manifest;
pub mod names;
pub mod scaffold;

pub use discover::{FileKind, Project, ProjectFile, find_root, load_project};
pub use manifest::{Manifest, Signing, parse_manifest};

/// File name of the project manifest.
pub const MANIFEST_FILE: &str = "era.toml";
/// Required entry point, relative to the project root.
pub const ENTRY_FILE: &str = "src/app.era";
