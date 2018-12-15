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

pub use crate::primitive::Prim;
pub use crate::scope::Scope;
pub use crate::with_error::WithError;
pub use crate::apply::Apply;
pub use crate::namespace::{INamespace, Namespace, SharedNamespace};

pub use crate::typing::*;

pub use crate::interpreter::*;
