//! Era Builder: a Framer-style visual app builder for ERA (D-095).
//!
//! The canvas (`era_canvas`) draws every screen; every change made on it
//! is a precise edit of the `.era` source (`edit`), so the code stays the
//! single source of truth and exports to native Kotlin (and Swift later).

pub mod app;
pub mod edit;
