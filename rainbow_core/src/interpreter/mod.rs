mod value;
mod instruction;
mod emitter;
mod script;
mod machine;

pub use self::value::*;
pub use self::instruction::*;
pub use self::script::*;
pub use self::machine::*;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Block {
  machine: usize,
  pub ip: u32,
  pub size: u16,
  pub argc: u8,
}

impl Block {
  pub fn call<'a, V: Value + 'a>(&self, args: Vec<V>) -> Result<V, V::Error> {
    let machine: &mut Machine<'a, V> =
      unsafe { &mut *(self.machine as *mut Machine<'a, V>) };
    machine.eval_block(self, args)
  }
}
