use std::collections;
use std::fmt;

use crate::frontend; //::{parse, NodeData, ParseError, SyntaxTree};
use crate::interpreter::{emitter, Instruction, Value};
use crate::namespace;
use crate::typing; //::{type_of, Type, TypeError};
use id_tree;
use pest;

pub struct Script<'i, V: Value> {
    pub ns: namespace::SharedNamespace<V>,
    pub tree: frontend::SyntaxTree<'i>,
    pub instructions: Vec<Instruction>,
    pub typer_result: typing::TypeCheckerResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    Parse,
    TypeCheck,
    Emit,
}

#[derive(Debug)]
pub enum CompileError<'i> {
    ParseError(pest::Error<'i, frontend::Rule>),
    NodeIdError(Stage, id_tree::NodeIdError),
}

impl<'i> From<frontend::ParseError<'i>> for CompileError<'i> {
    fn from(err: frontend::ParseError<'i>) -> Self {
        use crate::frontend::ParseError::*;
        match err {
            NodeId(err) => CompileError::NodeIdError(Stage::Parse, err),
            Pest(err) => CompileError::ParseError(err),
        }
    }
}

impl<'i> fmt::Display for CompileError<'i> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::CompileError::*;
        match *self {
            NodeIdError(stage, ref _err) => write!(f, "Internal compiler error stage={:?}", stage),
            ParseError(ref err) => write!(f, "{}", err),
        }
    }
}

impl<'i, V: Value> Script<'i, V> {
    pub fn compile(
        ns: namespace::SharedNamespace<V>,
        src: &'i str,
    ) -> Result<Self, CompileError<'i>> {
        use std::iter::empty;
        let tree = frontend::parse(&*ns.borrow(), frontend::Rule::term, src)?;
        let typer_result = typing::type_of(&*ns.borrow(), empty(), &tree);

        let instructions =
            emitter::emit(&tree).map_err(|err| CompileError::NodeIdError(Stage::Emit, err))?;

        Ok(Script {
            ns: ns.clone(),
            tree: tree,
            instructions: instructions,
            typer_result: typer_result,
        })
    }

    pub fn eval(&self, inputs: collections::HashMap<String, V>) -> Result<V, V::Error> {
        use crate::interpreter::machine::Machine;
        // use interpreter::interpreter::*;
        let bindings: Vec<_> = inputs
            .into_iter()
            .filter_map(|(name, value)| self.tree.symbols.find(&name).map(|id| (id, value)))
            .collect();

        let ns = self.ns.borrow();
        let mut machine = Machine::new(
            &*ns,
            &self.instructions,
            self.tree.constants.as_slice(),
            self.tree.symbols.as_slice(),
            bindings,
        );

        machine.run()
    }
}

#[cfg(test)]
mod tests {
    use super::Script;
    use crate::standalone::Value;
    use crate::test_helpers::*;
    use std::collections::HashMap;
    use std::iter::FromIterator;

    #[test]
    fn test_function_call() {
        let ns = init_namespace().into_shared();
        let script = Script::compile(ns, "calc: 1 plus: 2").unwrap();
        let result = script.eval(HashMap::new()).unwrap();
        assert_eq!(Value::from(3f64), result);
    }

    #[test]
    fn test_block() {
        let ns = init_namespace().into_shared();
        let script = Script::compile(ns, "each: [1 2 3] do: {x => x}").unwrap();
        let result = script.eval(HashMap::new()).unwrap();
        assert_eq!(
            Value::from_iter((1..4).map(|n| Value::from(n as f64))),
            result
        );
    }
}
