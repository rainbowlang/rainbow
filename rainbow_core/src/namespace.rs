use std::cell::RefCell;
use std::rc::Rc;

use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::apply::Apply;
use crate::arena::*;
use crate::function_builder::FunctionBuilder;
use crate::interpreter::{Machine, Value};
use crate::signature::Signature;

pub type SharedNamespace<V> = Rc<RefCell<Namespace<V>>>;

pub trait INamespace {
    fn new_empty() -> Self;
    fn get_signature(&self, name: &str) -> Option<&Signature>;
    fn symbols(&self) -> &Arena<String>;
}

#[derive(Serialize, Deserialize)]
pub struct Namespace<V: Value> {
    signatures: HashMap<ArenaId, Signature>,
    symbols: Arena<String>,
    #[serde(skip_serializing, skip_deserializing, default = "HashMap::new")]
    callbacks: HashMap<ArenaId, Box<Fn(Apply<V>, &mut Machine<V>) -> Result<V, V::Error>>>,
}

impl<V: Value> Default for Namespace<V> {
    fn default() -> Self {
        Namespace::new_empty()
    }
}

impl<V: Value> PartialEq for Namespace<V> {
    fn eq(&self, other: &Namespace<V>) -> bool {
        self.signatures == other.signatures
    }
}

impl<V: Value> Debug for Namespace<V> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", self.signatures)
    }
}

impl<V: Value> INamespace for Namespace<V> {
    fn new_empty() -> Self {
        Namespace {
            signatures: HashMap::new(),
            callbacks: HashMap::new(),
            symbols: Arena::with_capacity(256),
        }
    }

    fn get_signature(&self, name: &str) -> Option<&Signature> {
        self.symbols
            .find(&name)
            .and_then(|name| self.signatures.get(&name))
    }

    fn symbols(&self) -> &Arena<String> {
        &self.symbols
    }
}

impl<V: Value> Namespace<V> {
    pub fn new_with_prelude() -> Result<Self, String> {
        use crate::prelude;
        let mut ns = Self::new_empty();
        prelude::install(&mut ns)?;
        Ok(ns)
    }

    pub fn into_shared(self) -> SharedNamespace<V> {
        Rc::new(RefCell::new(self))
    }

    pub fn iter<'a>(&'a self) -> ::std::collections::hash_map::Iter<'a, ArenaId, Signature> {
        self.signatures.iter()
    }

    #[inline]
    pub fn get_callback(
        &self,
        id: &ArenaId,
    ) -> Option<&Box<Fn(Apply<V>, &mut Machine<V>) -> Result<V, V::Error>>> {
        self.callbacks.get(id)
    }

    pub fn intern_symbol(&mut self, s: &str) -> ArenaId {
        self.symbols.intern(s)
    }

    pub fn lookup_symbol(&self, id: ArenaId) -> &String {
        self.symbols.resolve(id)
    }

    pub fn insert(
        &mut self,
        signature: Signature,
        callback: Box<Fn(Apply<V>, &mut Machine<V>) -> Result<V, V::Error>>,
    ) -> Result<(), String> {
        let name = signature.name();
        if self.signatures.contains_key(&name) {
            return Err(format!(
                "function `{}` already defined",
                self.symbols().resolve(name)
            ));
        }
        self.signatures.insert(name, signature);
        self.callbacks.insert(name, callback);
        Ok(())
    }

    pub fn define<F: Fn(&mut FunctionBuilder<V>) -> ()>(&mut self, f: F) -> Result<(), String> {
        let (signature, callback) = {
            let mut builder: FunctionBuilder<V> = FunctionBuilder::new(&mut self.symbols);
            f(&mut builder);
            builder.into_parts()?
        };
        self.insert(signature, callback)
    }
}
