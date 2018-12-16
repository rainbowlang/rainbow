use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::completion::Completer;

use rainbow_core::{INamespace, Namespace, Scope, Script, SharedNamespace, Type, TypeCheckerResult};
use rainbow_core::standalone::Value;

use rainbow_core;

pub struct REPL {
    ns: SharedNamespace<Value>,
    env: RefCell<HashMap<String, (Value, Type)>>,
}

impl REPL {
    pub fn new() -> REPL {
        REPL {
            ns: Namespace::new_with_prelude().unwrap().into_shared(),
            env: RefCell::new(HashMap::new()),
        }
    }

    pub fn type_of(&self, input: &str) -> Result<TypeCheckerResult, String> {
        Script::compile(self.ns.clone(), input)
            .map_err(|err| format!("{}", err))
            .map(|script| script.typer_result)
    }

    pub fn eval(&self, input: &str) -> Result<(Value, Type), String> {
        Script::compile(self.ns.clone(), input)
            .map_err(|err| format!("{}", err))
            .and_then(|script| {
                let env = &*self.env.borrow();
                script
                    .eval(env.clone().into_iter().map(|(k, (v, t))| (k, v)).collect())
                    .map(|v| (v, script.typer_result.output))
            })
    }

    pub fn set(&self, name: &str, val: Value, ty: Type) {
        self.env.borrow_mut().insert(String::from(name), (val, ty));
    }
}

fn main() {
    let mut reader: Editor<Rc<REPL>> = Editor::new();

    let hist_path = std::env::home_dir().map(|path| path.join(".rainbow_history"));

    let save_history = |hist: &mut ::rustyline::history::History| {
        if let Some(ref path) = hist_path {
            hist.save(path)
                .unwrap_or_else(|e| panic!("cannot save {:?}: {}", path, e));
        }
    };

    if let Some(ref path) = hist_path {
        let hist = reader.get_history();
        let _ = hist.load(&path);
        save_history(hist);
    }

    let repl: Rc<REPL> = Rc::new(REPL::new());

    reader.set_completer(Some(repl.clone()));

    println!("This is Rainbow (press Ctrl-D to exit)");
    println!("");

    loop {
        let mut line = match reader.readline("🌈 ") {
            Ok(line) => line,
            Err(ReadlineError::Eof) => break,
            Err(error) => panic!("{:?}", error),
        };

        if line.trim().is_empty() {
            continue;
        } else {
            let hist = reader.get_history();
            hist.add(&line);
            save_history(hist);
        }

        if !line.starts_with(":") {
            line = format!(":eval {}", line);
        }

        let (cmd, rest) = match line.trim().find(|ch: char| ch.is_whitespace()) {
            Some(pos) => (&line[..pos], line[pos..].trim_left()),
            None => (line.as_str(), ""),
        };

        match cmd {
            ":eval" => {
                repl.eval(rest)
                    .map(|(val, ty)| println!("{} ~ {}", val, ty))
                    .unwrap_or_else(|e| println!("{}", e));
            }
            ":type" => match repl.type_of(rest) {
                Ok(result) => {
                    println!("{} ~ {}", rest, result.output);
                    let mut printed_globals_header = false;
                    for (ref name, ref ty) in result.inputs {
                        if repl.env.borrow().contains_key(name) {
                            continue;
                        }
                        if !printed_globals_header {
                            println!("\nInferred inputs:");
                            printed_globals_header = true;
                        }
                        println!("{} ~ {}", name, ty);
                    }
                }
                Err(message) => println!("{}", message),
            },
            ":func" => match repl.ns.borrow().get_signature(rest) {
                None => println!("`{}` is not defined", rest),
                Some(sig) => println!("{}", sig),
            },
            ":vars" => for (name, (val, ty)) in repl.as_ref().env.borrow().iter() {
                println!("{} = {} :: {}", name, val, ty);
            },
            ":set" => match rest.trim().find(|ch: char| ch.is_whitespace()) {
                None => {
                    println!(":set needs a variable name and expression. E.g. `:set foo \"hello\"");
                }
                Some(pos) => {
                    repl.eval(rest[pos..].trim_left())
                        .map(|(val, ty)| {
                            let name = &rest[..pos];
                            println!("{} = {} ~ {}", name, val, ty);
                            repl.set(name, val, ty);
                        })
                        .unwrap_or_else(|e| println!("{}", e));
                }
            },
            _ => println!("unknown command: {}", cmd),
        }
    }
}

impl Completer for REPL {
    fn complete(&self, line: &str, pos: usize) -> ::rustyline::Result<(usize, Vec<String>)> {
        use std::collections::BTreeSet;
        use std::iter::FromIterator;
        use rustyline::completion::extract_word;

        let break_chars = BTreeSet::from_iter(vec![' ', '\t', '[', '{'].into_iter());
        let (start, word) = extract_word(&line, pos, &break_chars);

        if start == 0 && word.starts_with(":") {
            let matches = vec![":eval", ":type", ":vars", ":func"]
                .into_iter()
                .filter_map(|cmd| {
                    if cmd.starts_with(word) {
                        Some(String::from(cmd))
                    } else {
                        None
                    }
                });
            return Ok((start, matches.collect()));
        }

        let buf: Vec<char> = line.chars().collect();
        let mut is_arg_name = true;
        let mut cursor = start;
        // scan backwards
        while cursor > 0 {
            cursor -= 1;
            match buf[cursor] {
                ':' | '>' | '[' | '{' => {
                    is_arg_name = false;
                    break;
                }
                ' ' | '\t' | '\r' | '\n' => {
                    continue;
                }
                _ => {
                    break;
                }
            }
        }

        if !is_arg_name || cursor == 0 {
            // at beginning of input or following a colon, we complete function names
            let ns = &*self.ns.borrow();
            let completions: Vec<String> = ns.iter()
                .filter_map(|(func_id, _)| {
                    let func_name = ns.lookup_symbol(*func_id);
                    if func_name.starts_with(word) {
                        Some(func_name.clone())
                    } else {
                        None
                    }
                })
                .collect();

            return Ok((cursor, completions));
        }

        // TODO - complete argument names for the current function
        //
        // there was a bunch of shitty code here that didn't work to find the name of the current function
        // it doesn't work because (I think) I need to integrate the actual parser here.
        Ok((pos, Vec::new()))
    }
}
