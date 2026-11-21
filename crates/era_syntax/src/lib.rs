//! ERA syntax: lexer, parser, AST and structural checks.
//!
//! The grammar is a single uniform node shape (see docs/GRAMMAR.md). This crate
//! checks *structure* (which forms may appear where, duplicate names, naming
//! conventions). Token, property and asset validation belongs to the IR stage.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod structure;

pub use ast::*;
pub use lexer::{Token, TokenKind, lex};
pub use parser::parse;
pub use structure::{SourceKind, SourceUnit, check_structure};
