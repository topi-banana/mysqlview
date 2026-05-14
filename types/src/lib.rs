//! Shared DTOs between the mysqlview backend and frontend.
//!
//! Kept dependency-light (only `serde` and `serde_json`) so it compiles cleanly
//! on both `wasm32-unknown-unknown` (frontend) and the server target.

pub mod browse;
pub mod ddl;
pub mod edit;
pub mod error;
pub mod query;
pub mod schema;

pub use browse::*;
pub use ddl::*;
pub use edit::*;
pub use error::*;
pub use query::*;
pub use schema::*;
