use crate::interpreter::Value;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use super::substitution::*;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Type {
    Any,
    Never,
    Num,
    Str,
    Bool,
    Time,
    Money,
    List(Box<Type>),
    Record(bool, HashMap<String, RecordField>),
    Block(Vec<Type>, Box<Type>),
    Var(String),
}

impl Type {
    pub fn quoted(t: Type) -> Type {
        Type::Block(vec![], Box::new(t))
    }

    pub fn block_from_to(inputs: Vec<Type>, output: Type) -> Type {
        Type::Block(inputs, Box::new(output))
    }

    pub fn list_of(t: Type) -> Type {
        Type::List(Box::new(t))
    }

    pub fn record_from_iter<K: Into<String>, T: IntoIterator<Item = (K, Type)>>(i: T) -> Type {
        //use std::iter::FromIterator;
        Type::Record(
            false,
            i.into_iter()
                .map(|(name, ty)| {
                    (
                        name.into(),
                        RecordField {
                            optional: false,
                            ty: ty,
                        },
                    )
                })
                .collect(),
        )
    }

    pub fn record_from_map(map: HashMap<String, RecordField>) -> Type {
        Type::Record(false, map)
    }

    pub fn record_with_one_field<S: Into<String>>(name: S, ty: Type, optional: bool) -> Type {
        let mut fields = HashMap::new();
        fields.insert(name.into(), RecordField::new(ty, optional));
        Type::Record(true, fields)
    }

    pub fn var(name: &str) -> Type {
        Type::Var(name.into())
    }

    pub fn var_name(&self) -> Option<&String> {
        match *self {
            Type::Var(ref name) => Some(name),
            _ => None,
        }
    }

    /// Perform run-time type-checking of a value
    pub fn satisfied_by_value<V: Value>(&self, value: &V) -> Result<(), Vec<V::Error>> {
        let mut errors = Vec::with_capacity(10);
        self.satisfied_by_inner(value, &mut errors, "".into());
        if errors.len() == 0 {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn satisfied_by_inner<V: Value>(&self, value: &V, errors: &mut Vec<V::Error>, prefix: String) {
        use crate::interpreter::Record;
        use crate::Type::*;
        match *self {
            Any => {}
            Never => errors.push(V::Error::from(format!("{}unexpected value", prefix))),
            Var(_) => {}
            Num => {
                if let Err(err) = value.try_number() {
                    errors.push(V::Error::from(format!("{}{}", prefix, err)));
                }
            }
            Str => {
                if let Err(err) = value.try_string() {
                    errors.push(V::Error::from(format!("{}{}", prefix, err)));
                }
            }
            Bool => {
                if let Err(err) = value.try_bool() {
                    errors.push(V::Error::from(format!("{}{}", prefix, err)));
                }
            }
            Time => {
                if let Err(err) = value.try_time() {
                    errors.push(V::Error::from(format!("{}{}", prefix, err)));
                }
            }
            Money => {
                errors.push(V::Error::from("money type is not ready yet".to_string()));
            }
            List(ref t) => match value.try_list() {
                Ok(list) => {
                    for (i, item) in list.into_iter().enumerate() {
                        t.satisfied_by_inner(&item, errors, format!("{}item {}: ", prefix, i));
                    }
                }
                Err(err) => {
                    errors.push(err);
                }
            },
            Record(_partial, ref fields) => match value.try_record() {
                Ok(record) => {
                    for (name, field) in fields {
                        match (record.at(name), field.optional) {
                            (Some(ref val), _) => {
                                field.ty.satisfied_by_inner(
                                    val,
                                    errors,
                                    format!("{}field `{}` ", prefix, name),
                                );
                            }
                            (None, false) => {
                                errors.push(V::Error::from(format!("field `{}` is required", name)))
                            }
                            (None, true) => (),
                        }
                    }
                }
                Err(err) => {
                    errors.push(err);
                }
            },
            Block(_, _) => {
                if !value.callable() {
                    errors.push(V::Error::from("not a block".to_string()))
                }
            }
        }
    }
}

impl ::std::default::Default for Type {
    fn default() -> Type {
        Type::Any
    }
}

impl Substitutable for Type {
    fn apply_substitution(&self, subs: &Subst) -> Self {
        match *self {
            Type::Block(ref inputs, ref output) => Type::Block(
                inputs
                    .into_iter()
                    .map(|t| t.apply_substitution(subs))
                    .collect(),
                Box::new(output.apply_substitution(subs)),
            ),
            Type::Record(partial, ref fields) => Type::Record(
                partial,
                fields
                    .iter()
                    .map(|(name, field)| {
                        (
                            name.clone(),
                            field.mutate_type(|ty| ty.apply_substitution(subs)),
                        )
                    })
                    .collect(),
            ),
            Type::Var(ref name) => match subs.get(name) {
                Some(t) => t.clone(),
                None => Type::Var(name.clone()),
            },
            Type::List(ref element) => Type::List(Box::new(element.apply_substitution(subs))),
            _ => self.clone(),
        }
    }

    fn free_vars(&self) -> Option<HashSet<String>> {
        match *self {
            Type::Block(ref inputs, ref output) => {
                extend_vars(inputs.iter().fold(None, extend_vars), output.as_ref())
            }
            Type::Var(ref name) => {
                let mut vars: HashSet<String> = HashSet::new();
                vars.insert(name.clone());
                Some(vars)
            }
            Type::List(ref element) => element.free_vars(),
            Type::Record(_partial, ref fields) => fields
                .iter()
                .fold(None as Option<HashSet<String>>, |vars, (_, field)| {
                    extend_vars(vars, field.get_type())
                }),
            _ => None,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter) -> Result<(), ::std::fmt::Error> {
        use self::Type::*;
        use std::fmt::Write;

        match *self {
            Any => f.write_str("any"),
            Never => f.write_str("never"),
            Num => f.write_str("number"),
            Str => f.write_str("string"),
            Bool => f.write_str("boolean"),
            Time => f.write_str("time"),
            Money => f.write_str("money"),
            List(ref t) => write!(f, "[ {}... ]", t),
            Var(ref name) => f.write_str(name),
            Record(partial, ref fields) => {
                f.write_char('[')?;
                if partial {
                    f.write_char('?')?;
                }

                let mut field_vec: Vec<(&String, &RecordField)> = fields.iter().collect();
                field_vec.sort_by(|&(name1, f1), &(name2, f2)| -> Ordering {
                    if f1.optional == f2.optional {
                        name1.cmp(name2)
                    } else if f1.optional {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                });

                for &(name, field) in field_vec.iter() {
                    write!(f, " {}", name)?;
                    if field.optional {
                        f.write_char('?')?;
                    };
                    write!(f, "={}", field.ty)?;
                }
                f.write_char(' ')?;
                if partial {
                    f.write_char('?')?;
                }
                f.write_char(']')
            }
            Block(ref inputs, ref output) => {
                f.write_str("{ ")?;
                if inputs.len() > 0 {
                    for input in inputs.iter() {
                        write!(f, "{} ", input)?;
                    }
                    f.write_str("=> ")?;
                }
                write!(f, "{} }}", output)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct RecordField {
    optional: bool,
    ty: Type,
}

impl RecordField {
    pub fn new(ty: Type, optional: bool) -> RecordField {
        RecordField {
            ty: ty,
            optional: optional,
        }
    }

    pub fn required(&self) -> bool {
        !self.optional
    }

    pub fn optional(&self) -> bool {
        self.optional
    }

    pub fn get_type(&self) -> &Type {
        &self.ty
    }

    pub fn map_type<F: Fn(Type) -> Type>(self, f: F) -> Self {
        RecordField {
            ty: f(self.ty),
            optional: self.optional,
        }
    }

    pub fn mutate_type<F: Fn(&Type) -> Type>(&self, f: F) -> Self {
        RecordField {
            ty: f(&self.ty),
            optional: self.optional,
        }
    }
}

#[macro_export]
macro_rules! record_type {
    ($($name:expr => $field:expr),*) => {
        {
            let mut fields: HashMap<String, RecordField> = HashMap::new();
            $(fields.insert(String::from($name), $field);)*
            Type::record_from_map(fields)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::standalone::Value;
    use crate::typing::types::*;
    use std::collections::HashMap;

    #[test]
    fn record_display() {
        let t3 = record_type!(
            "foo" => RecordField::new(Type::Num, false),
            "Wat" => RecordField::new(Type::list_of(Type::Num), true)
        );
        assert_eq!(format!("{}", t3), "[ foo=number Wat?=[ number... ] ]")
    }

    #[test]
    fn test_satisfied_by_primitive_types() {
        let pairs = vec![
            (Type::Num, Value::from(3f64)),
            (Type::Str, Value::from("hello")),
            (Type::Bool, Value::from(false)),
            (Type::Time, Value::from(3u64)),
        ];
        for (i, &(ref ty, ref val)) in (&pairs).into_iter().enumerate() {
            assert_eq!(ty.satisfied_by_value(val), Ok(()));
            for (j, &(_, ref wrong_val)) in (&pairs).into_iter().enumerate() {
                if i != j {
                    assert!(ty.satisfied_by_value(wrong_val).is_err());
                }
            }
        }
    }

    #[test]
    fn test_satisfied_by_lists() {
        use std::iter::FromIterator;

        let list = Value::from_iter(vec![Value::from(3f64)]);
        assert_eq!(Type::list_of(Type::Num).satisfied_by_value(&list), Ok(()));

        assert_eq!(
            Type::list_of(Type::Str).satisfied_by_value(&list),
            Err(vec!["item 0: 3 is not a string".into()])
        );
    }

    #[test]
    fn test_satisfied_by_records() {
        use std::iter::FromIterator;

        let ty = Type::record_from_iter(vec![("datnum", Type::Num)]);
        let good = Value::from_iter(vec![("datnum".to_string(), Value::from(3f64))]);
        let bad = Value::from_iter(vec![("datnum".to_string(), Value::from("Yo"))]);
        assert_eq!(ty.satisfied_by_value(&good), Ok(()));
        assert_eq!(
            ty.satisfied_by_value(&bad),
            Err(vec!["field `datnum` \"Yo\" is not a number".to_string()])
        );
        let bad_missing_field = Value::from_iter(vec![("blah".to_string(), Value::from(false))]);
        assert_eq!(
            ty.satisfied_by_value(&bad_missing_field),
            Err(vec!["field `datnum` is required".to_string()])
        );
    }
}
