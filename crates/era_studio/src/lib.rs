//! ERA Studio: a desktop editor for ERA projects (M7).
//!
//! The UI (`app`) is a thin layer over testable pieces: the open project
//! and its buffers (`workspace`), ERA syntax colouring (`highlight`) and
//! background `era preview` / `era build android` runs (`jobs`).

pub mod app;
pub mod highlight;
pub mod jobs;
pub mod workspace;
