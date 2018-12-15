#[cfg(test)]
use std::iter::empty;

use typing;
use typing::Type;

use test_helpers::*;

fn type_of<G: IntoIterator<Item = (String, Type)>>(
  expr: &str,
  globals: G,
) -> typing::TypeCheckerResult {
  let functions = init_namespace();
  typing::type_of(&functions, globals, &parse(&functions, expr))
}

#[test]
fn inference_of_primitives() {
  assert_eq!(type_of("1", vec![]).unwrap().0, Type::Num)
}

#[test]
fn inference_of_variables() {
  assert_eq!(type_of("foo", vec![]).unwrap().0, Type::var("$1"));
  assert_eq!(
    type_of("bar", vec![("bar".to_string(), Type::Num)])
      .unwrap()
      .0,
    Type::Num
  );
}

#[test]
fn inference_of_lists() {}

#[test]
fn inference_of_blocks() {
  assert_eq!(
    type_of("{ foo }", vec![]).unwrap().0,
    Type::block_from_to(vec![], Type::var("$1"))
  );
  let globals = vec![("bar".to_string(), Type::Num)];
  assert_eq!(
    type_of("{ bar }", globals).unwrap().0,
    Type::block_from_to(vec![], Type::Num)
  );
}

#[test]
fn inference_of_functions_and_undefined_vars() {
  let code = "each: [ foo bar ] do: { it => calc: it.cost times: it.quantity times: baz }";
  let (ty, inferred_env) = type_of(code, vec![]).unwrap();

  let expected_record_type = Type::record_from_iter(vec![
    ("cost".to_string(), Type::Num),
    ("quantity".to_string(), Type::Num),
  ]);

  assert_eq!(ty, Type::list_of(Type::Num));
  assert_eq!(inferred_env.get("foo").unwrap(), &expected_record_type);

  assert_eq!(inferred_env.get("bar"), inferred_env.get("foo"));

  assert_eq!(inferred_env.get("baz").unwrap(), &Type::Num);
}

#[test]
fn merging_of_nested_records() {
  let (ty, inferred_env) = type_of("calc: foo.bar.baz plus: foo.bar.qux", vec![]).unwrap();
  assert_eq!(ty, Type::Num);
  let type_foo_bar = Type::record_from_iter(vec![("baz", Type::Num), ("qux", Type::Num)]);
  let type_foo = Type::record_from_iter(vec![("bar", type_foo_bar)]);
  assert_eq!(
    inferred_env.into_iter().collect::<Vec<_>>(),
    vec![("foo".to_string(), type_foo)],
  );
}

#[test]
fn bigger_example() {
  let ty_lat_lon = Type::record_from_iter(vec![("lat", Type::Num), ("lon", Type::Num)]);
  let ty_promo = Type::record_from_iter(vec![("name", Type::Str)]);
  let ty_partner = Type::record_from_iter(vec![
    ("name", Type::Str),
    ("current_promos", Type::list_of(ty_promo.clone())),
  ]);
  let location = Type::record_from_iter(vec![
    ("name", Type::Str),
    ("street_address", Type::Str),
    ("lat_lon", ty_lat_lon.clone()),
    ("partner", ty_partner.clone()),
  ]);

  let mut ns = init_namespace();
  ns.define({
    let ty_lat_lon = ty_lat_lon.clone();
    move |f| {
      f.required_arg("nearby", ty_lat_lon.clone());
      f.returns(Type::list_of(location.clone()));
      f.callback(|_args, _vm| Err(String::from("unimplemented")));
    }
  }).unwrap();
  let stx = parse(
    &ns,
    "each: { nearby: here } do: { it => [
                name = it.partner.name
                address = it.street_address
                promos = each: it.partner.current_promos do: { p => p.name }
            ]}",
  );
  let (ty, inferred_env) = typing::type_of(&ns, empty(), &stx).unwrap();
  assert_eq!(
    inferred_env.into_iter().collect::<Vec<_>>(),
    vec![("here".into(), ty_lat_lon.clone())],
  );
  assert_eq!(
    ty,
    Type::list_of(Type::record_from_iter(vec![
      ("name", Type::Str),
      ("address", Type::Str),
      ("promos", Type::list_of(Type::Str)),
    ]))
  );
}
