use crate::interpreter::{Block, Instruction, Value};
use crate::namespace::Namespace;
use crate::primitive::Prim;

pub struct Machine<'a, V: Value + 'a> {
    ns: &'a Namespace<V>,
    instructions: &'a [Instruction],
    instruction_pointer: usize,
    program_data: &'a [Prim],
    pub symbols: &'a [String],
    bindings: Vec<(u16, V)>,
    value_stack: Vec<V>,
    keyword_stack: Vec<u16>,
}

#[derive(Debug)]
pub enum MachineError {
    ValueStackEmpty,
    KeywordStackEmpty,
    Undefined,
}

impl<'a, V: Value + 'a> Machine<'a, V> {
    pub fn new(
        ns: &'a Namespace<V>,
        instructions: &'a [Instruction],
        program_data: &'a [Prim],
        symbols: &'a [String],
        bindings: Vec<(u16, V)>,
    ) -> Self {
        Machine {
            ns: ns,
            instructions: instructions,
            instruction_pointer: 0,
            program_data: program_data,
            symbols: symbols,
            bindings: bindings,
            value_stack: Vec::with_capacity(128),
            keyword_stack: Vec::with_capacity(32),
        }
    }

    pub fn run(&mut self) -> Result<V, V::Error> {
        self.eval_range(0, self.instructions.len())?;
        self.pop_value()
    }

    fn eval_range(&mut self, start: usize, count: usize) -> Result<(), V::Error> {
        let old_ip = self.instruction_pointer;
        self.instruction_pointer = start;
        let end = start + count;
        loop {
            if self.instruction_pointer >= end {
                break;
            }
            self.step()?;
        }
        self.instruction_pointer = old_ip;
        Ok(())
    }

    /*
    pub fn eval_block(&mut self, block: &Block, args: Vec<V>) -> Result<V, V::Error> {
      let orig_value_stack_size = self.value_stack.len();
      let orig_keyword_stack_size = self.keyword_stack.len();
      let orig_bindings_size = self.bindings.len();
      let orig_ip = self.instruction_pointer;

      self.value_stack.extend(args);
      self.instruction_pointer = block.ip as usize;
      self.eval_range(block.ip as usize, block.size as usize)?;

      let result = self.pop_value();

      self.instruction_pointer = orig_ip;
      self.value_stack.truncate(orig_value_stack_size);
      self.keyword_stack.truncate(orig_keyword_stack_size);
      self.bindings.truncate(orig_bindings_size);
      result
    }
    */

    pub fn eval_block(&mut self, block: &Block, args: Vec<V>) -> Result<V, V::Error> {
        let orig_value_stack_size = self.value_stack.len();
        let orig_keyword_stack_size = self.keyword_stack.len();
        let orig_bindings_size = self.bindings.len();
        let orig_ip = self.instruction_pointer;

        self.value_stack.extend(args);
        self.instruction_pointer = block.ip as usize;
        self.eval_range(block.ip as usize, block.size as usize)?;

        let result = self.pop_value();

        self.instruction_pointer = orig_ip;
        self.value_stack.truncate(orig_value_stack_size);
        self.keyword_stack.truncate(orig_keyword_stack_size);
        self.bindings.truncate(orig_bindings_size);
        result
    }

    fn step(&mut self) -> Result<(), V::Error> {
        use crate::interpreter::Instruction::*;

        match self.instructions[self.instruction_pointer] {
            PushPrimitive { id } => self
                .value_stack
                .push(box_prim(&self.program_data[id as usize])),
            PushVar { id } => {
                let value = self
                    .bindings
                    .iter()
                    .rev()
                    .filter_map(|&(sym_id, ref val)| {
                        if sym_id == id {
                            Some(val.clone())
                        } else {
                            None
                        }
                    })
                    .next()
                    .ok_or_else(|| self.error(MachineError::Undefined))?;
                self.value_stack.push(value);
            }
            PushProp { id } => {
                use crate::interpreter::Record;
                let record = self
                    .value_stack
                    .pop()
                    .ok_or_else(|| self.error(MachineError::ValueStackEmpty))?
                    .try_record()?;
                let value = record
                    .at(&self.symbols[id as usize])
                    .ok_or_else(|| self.error(MachineError::Undefined))?;
                self.value_stack.push(value);
            }
            PushKeyword { id } => {
                self.keyword_stack.push(id);
            }
            MkList { size } => {
                let value = { V::from_iter(self.pop_values(size)?) };
                self.value_stack.push(value);
            }
            MkRecord { size } => {
                let value = {
                    let name_value_pairs = self
                        .pop_pairs(size)?
                        .into_iter()
                        .map(|(sym_id, val)| (self.symbols[sym_id as usize].clone(), val));
                    V::from_iter(name_value_pairs)
                };
                self.value_stack.push(value);
            }
            MkBlock { argc, skip } => {
                let block = Block {
                    machine: (self as *mut Self as usize),
                    ip: (self.instruction_pointer + 1) as u32,
                    argc: argc,
                    size: skip,
                };
                self.instruction_pointer += skip as usize;
                self.value_stack.push(V::from(block));
            }
            Bind { id } => {
                let value = { self.pop_value()? };
                self.bindings.push((id, value));
            }
            CallFunction { argc } => {
                use crate::apply::Apply;
                let apply = Apply::from(self.pop_pairs(argc)?);
                let value = {
                    let func_id = apply.func_id().clone();
                    let callback = self.ns.get_callback(&func_id).ok_or_else(|| {
                        format!("Function `{}` is undefined", self.symbols[func_id as usize])
                    })?;
                    callback(apply, self)?
                };
                self.value_stack.push(value);
            }
        }
        self.instruction_pointer += 1;
        Ok(())
    }

    fn pop_value(&mut self) -> Result<V, V::Error> {
        self.value_stack
            .pop()
            .ok_or_else(|| self.error(MachineError::ValueStackEmpty))
    }

    fn pop_values(&mut self, count: u16) -> Result<Vec<V>, V::Error> {
        let top = self.value_stack.len();
        if (count as usize) > top {
            return Err(self.error(MachineError::ValueStackEmpty));
        }
        let values = self.value_stack.split_off(top - count as usize);
        Ok(values)
    }

    fn pop_pairs(&mut self, count: u16) -> Result<Vec<(u16, V)>, V::Error> {
        let stack_size = self.keyword_stack.len();
        if (count as usize) > stack_size {
            return Err(self.error(MachineError::KeywordStackEmpty));
        }
        let keywords = self.keyword_stack.split_off(stack_size - count as usize);
        let values = self.pop_values(count)?;
        Ok(keywords.into_iter().zip(values).collect())
    }

    fn error(&self, err: MachineError) -> V::Error {
        V::Error::from(format!(
            "{:?} @ instruction {} {:?}",
            err, self.instruction_pointer, self.instructions[self.instruction_pointer],
        ))
    }
}

fn box_prim<V: Value>(prim: &Prim) -> V {
    match *prim {
        Prim::Number(n) => V::from(n),
        Prim::String(ref s) => V::from(s.clone()),
        Prim::Boolean(b) => V::from(b),
        Prim::Time(i) => V::from(i),
        Prim::Money(_, _n) => panic!("no money"),
    }
}
