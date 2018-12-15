use crate::apply::Apply;
use crate::arena::ArenaId;
use crate::arena::*;
use crate::interpreter::{Machine, Value};
use crate::signature::{Argument, Signature};
use crate::typing::Type;

pub struct FunctionBuilder<'a, V: Value> {
    symbols: &'a mut Arena<String>,
    signature: Signature<ArenaId>,
    return_type: Option<Type>,
    callback: Option<Box<Fn(Apply<V>, &mut Machine<V>) -> Result<V, V::Error>>>,
}

impl<'a, V: Value> FunctionBuilder<'a, V> {
    pub fn new(symbols: &'a mut Arena<String>) -> Self {
        FunctionBuilder {
            symbols: symbols,
            signature: Signature::with_capacity(4),
            return_type: None,
            callback: None,
        }
    }

    pub fn required_arg(&mut self, name: &str, ty: Type) -> ArenaId {
        let name_id = self.symbols.intern(name);
        self.signature.add_argument(Argument {
            name: name_id,
            ty: ty.clone(),
            variadic: false,
            required: true,
        });
        name_id
    }

    pub fn optional_arg(&mut self, name: &str, ty: Type) -> ArenaId {
        let name_id = self.symbols.intern(name);
        self.signature.add_argument(Argument {
            name: name_id,
            ty: ty.clone(),
            variadic: false,
            required: false,
        });
        name_id
    }

    pub fn variadic_arg(&mut self, name: &str, ty: Type) -> ArenaId {
        let name_id = self.symbols.intern(name);
        self.signature.add_argument(Argument {
            name: name_id,
            ty: ty.clone(),
            variadic: true,
            required: false,
        });
        name_id
    }

    pub fn required_variadic_arg(&mut self, name: &str, ty: Type) -> ArenaId {
        let name_id = self.symbols.intern(name);
        self.signature.add_argument(Argument {
            name: name_id,
            ty: ty.clone(),
            variadic: true,
            required: true,
        });
        name_id
    }

    pub fn returns(&mut self, ty: Type) {
        self.return_type = Some(ty);
    }

    pub fn is_partial(&mut self) {
        self.signature.set_total(false);
    }

    pub fn is_total(&mut self) {
        self.signature.set_total(true);
    }

    pub fn callback<F>(&mut self, cb: F)
    where
        F: 'static + Fn(Apply<V>, &mut Machine<V>) -> Result<V, V::Error>,
    {
        self.callback = Some(Box::new(cb));
    }

    pub fn into_parts(
        self,
    ) -> Result<
        (
            Signature,
            Box<Fn(Apply<V>, &mut Machine<V>) -> Result<V, V::Error>>,
        ),
        String,
    > {
        let mut signature = self.signature;

        let returns = self.return_type.ok_or(format!(
            "def {}: return type must be defined",
            signature.name()
        ))?;
        let callback = self.callback.ok_or(format!(
            "def {}: callback must be defined",
            signature.name()
        ))?;

        signature.set_return_type(returns);

        Ok((signature, callback))
    }
}
