use crate::interpreter::Value;
use crate::namespace::Namespace;
use crate::typing::Type;

pub fn install<V: Value>(ns: &mut Namespace<V>) -> Result<(), String> {
    ns.define(|f| {
        let msg = f.required_arg("crash", Type::Str);
        f.returns(Type::Any);
        f.callback(move |args, _vm| {
            let v = args.demand(&msg)?;
            let s = v.try_string()?;
            Err(V::Error::from(String::from(s)))
        });
    })?;

    ns.define(|f| {
        let not = f.required_arg("not", Type::Bool);
        f.returns(Type::Bool);
        f.callback(move |args, _vm| {
            args.demand(&not)
                .and_then(|v| v.try_bool())
                .map(|b| (!b).into())
        });
    })?;

    ns.define(|f| {
        let if_ = f.required_arg("if", Type::Bool);
        let and = f.variadic_arg("and", Type::quoted(Type::Bool));
        let or = f.variadic_arg("or", Type::quoted(Type::Bool));
        let then = f.required_arg("then", Type::quoted(Type::var("A")));
        let else_ = f.required_arg("else", Type::quoted(Type::var("A")));

        f.returns(Type::Var("A".into()));
        f.callback(move |args, vm| {
            let mut yes_no: bool = args.demand(&if_)?.try_bool()?;
            for &(keyword, ref val) in args.iter().skip(1) {
                if yes_no && keyword == and {
                    yes_no = val.try_call(vm, vec![])?.try_bool()?;
                } else if !yes_no && keyword == or {
                    yes_no = val.try_call(vm, vec![])?.try_bool()?;
                } else if keyword == then || keyword == else_ {
                    break;
                }
            }

            args.demand(if yes_no { &then } else { &else_ })?
                .try_block()
                .and_then(|block| vm.eval_block(block, vec![]))
        });
    })?;

    ns.define(|f| {
        let cmp = f.required_arg("compare", Type::Num);
        let gt = f.optional_arg("biggerThan", Type::Num);
        let gte = f.optional_arg("atLeast", Type::Num);
        let lt = f.optional_arg("smallerThan", Type::Num);
        let lte = f.optional_arg("atMost", Type::Num);
        f.is_total();
        f.returns(Type::Bool);
        f.callback(move |args, _vm| {
            let it = args.demand(&cmp)?.try_number()?;
            for &(keyword, ref val) in args.iter().skip(1) {
                let other = val.try_number()?;
                let pass = if keyword == gt {
                    it > other
                } else if keyword == gte {
                    it >= other
                } else if keyword == lt {
                    it < other
                } else if keyword == lte {
                    it <= other
                } else {
                    true
                };
                if !pass {
                    return Ok(pass.into());
                }
            }
            Ok(true.into())
        });
    })?;

    ns.define(|f| {
        let each = f.required_arg("each", Type::list_of(Type::var("In")));
        let block_type = Type::block_from_to(vec![Type::var("In")], Type::var("Out"));
        let do_ = f.required_arg("do", block_type);
        f.returns(Type::list_of(Type::var("Out")));
        f.callback(move |args, vm| {
            let list = args.demand(&each)?.try_list()?;
            let block = args.demand(&do_)?.try_block()?;
            let out: Result<Vec<V>, V::Error> = list
                .into_iter()
                .map(|item| vm.eval_block(block, vec![item]))
                .collect();
            out.map(|vec| vec.into())
        });
    })?;

    ns.define(|f| {
        let r#try = f.required_arg("try", Type::quoted(Type::var("A")));
        let or = f.required_arg("or", Type::quoted(Type::var("A")));
        f.returns(Type::var("A"));
        f.callback(move |args, vm| {
            args.demand(&r#try)?
                .try_call(vm, vec![])
                .or_else(|_suppressed_error| args.demand(&or)?.try_call(vm, vec![]))
        });
    })?;

    ns.define(|f| {
        let sum = f.required_arg("sum", Type::list_of(Type::Num));
        f.returns(Type::Num);
        f.callback(move |args, _vm| {
            let list = args.demand(&sum)?.try_list()?;
            let mut sum = 0_f64;
            for item in list {
                sum += item.try_number()?;
            }
            Ok(sum.into())
        });
    })?;

    ns.define(|f| {
        let count_f = f.required_arg("countFrom", Type::Num);
        let to = f.required_arg("to", Type::Num);
        let by = f.optional_arg("by", Type::Num);
        f.returns(Type::list_of(Type::Num));
        f.callback(move |args, _vm| {
            let start = args.demand(&count_f)?.try_number()?;
            let mut step = args
                .demand(&by)
                .and_then(|v| v.try_number())
                .unwrap_or(1_f64);
            let end = args.demand(&to)?.try_number()?;

            if step.abs() < 0.00001_f64 {
                step = 1_f64;
            }

            if (start > end && step.is_sign_positive()) || (start < end && step.is_sign_negative())
            {
                step *= -1_f64;
            }

            let expected_size = ((end - start) / step).ceil() as usize;
            let mut out: Vec<V> = Vec::with_capacity(expected_size);
            let mut here = start;
            while here <= end {
                out.push(here.into());
                here += step;
            }
            Ok(out.into())
        });
    })?;

    ns.define(|f| {
        let calc = f.required_arg("calc", Type::Num);
        let add = f.variadic_arg("plus", Type::Num);
        let sub = f.variadic_arg("subtract", Type::Num);
        let mul = f.variadic_arg("times", Type::Num);
        let div = f.variadic_arg("dividedBy", Type::Num);
        f.returns(Type::Num);
        f.callback(move |args, _vm| {
            let init = args.demand(&calc)?.try_number();
            args.iter()
                .skip(1)
                .fold(init, |result, &(keyword, ref val)| {
                    let r = result?;
                    let n = val.try_number()?;
                    Ok(if keyword == add {
                        r + n
                    } else if keyword == sub {
                        r - n
                    } else if keyword == mul {
                        r * n
                    } else if keyword == div {
                        r / n
                    } else {
                        r
                    })
                })
                .map(V::from)
        });
        f.is_partial(); // division by zero will fail
    })?;

    // with: 12 do: { x => calc: 1 add: x }
    ns.define(|f| {
        let with = f.required_arg("with", Type::var("In"));
        let block_type = Type::block_from_to(vec![Type::var("In")], Type::var("Out"));
        let do_ = f.required_arg("do", block_type);
        f.returns(Type::var("Out"));
        f.callback(move |args, vm| {
            let block = args.demand(&do_)?;
            block.try_call(vm, vec![args.demand(&with)?.clone()])
        });
    })?;

    ns.define(|f| {
        let upper = f.required_arg("upperCase", Type::Str);
        f.returns(Type::Str);
        f.callback(move |args, _vm| {
            let s = args.demand(&upper)?.try_string()?;
            Ok(V::from(s.to_uppercase()))
        });
    })?;

    ns.define(|f| {
        let stringify = f.required_arg("stringify", Type::var("Any"));
        f.returns(Type::Str);
        f.callback(move |args, _vm| {
            args.demand(&stringify)
                .map(|arg| format!("{:?}", arg))
                .map(V::from)
        });
    })?;

    Ok(())
}
