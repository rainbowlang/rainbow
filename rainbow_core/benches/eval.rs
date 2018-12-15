#[macro_use]
extern crate bencher;
extern crate rainbow_core;

use std::iter::FromIterator;

macro_rules! eval_benchmark {
  ($bench_name:ident, $rainbow_src:expr, $result:expr $(, $var_name:ident => $value:expr)*) => {
    fn $bench_name(b: &mut ::bencher::Bencher) {
      use std::collections::HashMap;
      use rainbow_core::{Namespace, Script};
      use rainbow_core::standalone::Value;
      let ns = Namespace::new_with_prelude().unwrap().into_shared();
      let script = Script::compile(ns, $rainbow_src).unwrap();
      let mut inputs: HashMap<String, Value> = HashMap::with_capacity(16);
      $(
        inputs.insert(
          String::from(stringify!($var_name)),
          Value::from($value),
        );
      )*
      let output = Value::from($result);
      b.iter(|| assert_eq!(output, script.eval(inputs.clone()).unwrap()));
    }
  };
}

eval_benchmark!(
  identity_iteration,
  "each: xs do: { x => x }",
  Value::from_iter((1..100).map(|x| Value::from(x as f64))),
  xs => Value::from_iter((1..100).map(|x| Value::from(x as f64)))
);

eval_benchmark!(math_with_literals, "calc: 2 plus: 2", 4f64);

eval_benchmark!(
  math_with_vars,
  "calc: x plus: y",
  12f64,
  x => 8f64,
  y => 4f64
);

eval_benchmark!(
  nested_iteration,
  "each: { countFrom: 1 to: end } do: { i =>
    each: { countFrom: 1 to: i } do: { j => calc: i times: j }
  }",
  Value::from_iter((1..101).map(|i| Value::from_iter((1..(i+1)).map(|j| Value::from((i * j) as f64))))),
  end => 100f64
);

benchmark_group!(
    benches,
    identity_iteration,
    math_with_literals,
    math_with_vars,
    nested_iteration
);
benchmark_main!(benches);
