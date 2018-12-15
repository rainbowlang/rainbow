use crate::frontend;
use crate::interpreter;
use crate::namespace::Namespace;
use crate::standalone::Value;

pub fn init_namespace() -> Namespace<Value> {
    Namespace::new_with_prelude().unwrap()
}

pub fn parse_with_prelude<'a>(src: &'a str) -> frontend::SyntaxTree<'a> {
    frontend::parse(&init_namespace(), frontend::Rule::term, src).unwrap()
}

pub fn parse<'i>(functions: &Namespace<Value>, expr: &'i str) -> frontend::SyntaxTree<'i> {
    frontend::parse(functions, frontend::Rule::term, expr).unwrap()
}

pub fn compile_with_prelude<'a>(src: &'a str) -> interpreter::Script<'a, Value> {
    let ns = init_namespace().into_shared();
    interpreter::Script::compile(ns, src).unwrap()
}
