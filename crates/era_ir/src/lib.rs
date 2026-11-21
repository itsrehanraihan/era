//! ERA semantic analysis and the platform-independent UI IR.
//!
//! Input: a loaded project plus parsed `.era`/`.ers` files (structure already
//! checked by `era_syntax`). Output: typed skin tokens, a fully resolved UI
//! tree per screen, and diagnostics. No Android (or any platform) types live
//! here; backends consume [`ir::App`].

pub mod ir;
pub mod logic;
pub mod pipeline;

pub mod skin;

pub mod resolve;
pub mod schema;
pub use resolve::{ParsedFile, build_app};
pub mod dump;
