extern crate id_tree;
// extern crate parity_wasm;
#[cfg_attr(test, macro_use)]
extern crate pest;
#[macro_use]
extern crate pest_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod macros;
mod arena;
mod with_error;
mod apply;
pub mod signature;
mod function_builder;
mod namespace;
mod typing;
mod scope;
mod primitive;
pub mod frontend;
pub mod interpreter;

#[cfg(test)]
pub mod test_helpers;

mod prelude;
pub mod standalone;

pub use primitive::Prim;
pub use scope::Scope;
pub use with_error::WithError;
pub use apply::Apply;
pub use namespace::{INamespace, Namespace, SharedNamespace};

pub use typing::*;

pub use interpreter::*;
