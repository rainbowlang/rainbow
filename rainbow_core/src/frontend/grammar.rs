#[derive(Parser)]
#[grammar = "frontend/grammar.pest"]
pub struct RainbowGrammar;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variables() {
        let variables = vec!["foo", "_neato", "foo_bar", "FooBar"];
        for input in variables.into_iter() {
            parses_to! {
                parser: RainbowGrammar,
                input: input,
                rule: Rule::term,
                tokens: [
                    variable(0, input.len(), [ident(0, input.len())])
                ]
            };

            let separators = vec![".", " .", ". ", " . "];
            for sep in separators.into_iter() {
                let path = vec![
                    String::from(input),
                    String::from(input),
                    String::from(input),
                ].join(sep);

                parses_to! {
                    parser: RainbowGrammar,
                    input: &path,
                    rule: Rule::term,
                    tokens: [
                        variable(0, path.len(), [
                            ident(0, input.len()),
                            ident(input.len() + sep.len(), input.len()*2 + sep.len()),
                            ident(input.len()*2 + sep.len()*2, path.len())
                        ])
                    ]
                };
            }
        }
    }

    #[test]
    fn test_parse_keyword() {
        parses_to! {
            parser: RainbowGrammar,
            input: "foo:",
            rule: Rule::keyword,
            tokens: [
                keyword(0, 4, [])
            ]
        };
    }

    #[test]
    fn test_parse_numbers() {
        let numbers = vec!["1", "1000", "1.5", "1.6e10", "1.6e-10", "100_000"];

        for input in numbers.iter() {
            parses_to! {
                parser: RainbowGrammar,
                input: input,
                rule: Rule::term,
                tokens: [number(0, input.len())]
            }
        }
    }

    #[test]
    fn test_parse_larger() {
        let src = "each: offices do: { office => [
            name = get_name: office
            employees = each: office.employees do: { e => get_name: e }
        ] }";
        parses_to! {
            parser: RainbowGrammar,
            input: src,
            rule: Rule::term,
            tokens: [
                apply(0, src.len(), [
                    argument(0, 14, [
                        keyword(0, 5), // each:
                        variable(6, 14, [
                            ident(6, 13) // offices
                        ]),
                    ]),
                    argument(14, src.len(), [
                        keyword(14, 17), // do:
                        block(18, src.len(), [
                            block_args(20, 29, [
                                ident(20, 26) // office
                            ]),
                            record(30, src.len() - 2, [
                                entry(44, 80, [
                                    ident(44, 48), // name
                                    apply(51, 80, [
                                        argument(51, 80, [
                                            keyword(51, 60),  // get_name:
                                            variable(61, 80, [
                                                ident(61, 67) // office
                                            ])
                                        ])
                                    ])
                                ]),
                                entry(80, 139, [
                                    ident(80, 89), // employees
                                    apply(92, 139, [
                                        argument(92, 114, [
                                            keyword(92, 97), // each:
                                            variable(98, 114, [
                                                ident(98, 104), // office
                                                ident(105, 114), // employees
                                            ]),
                                        ]),
                                        argument(115, 139, [
                                            keyword(115, 118), // do:
                                            block(119, 139, [
                                                block_args(121, 125, [
                                                    ident(121, 122), // e
                                                ]),
                                                apply(126, 138, [
                                                    argument(126, 138, [
                                                        keyword(126, 135), // get_name:
                                                        variable(136, 138, [
                                                            ident(136, 137) // e
                                                        ])
                                                    ])
                                                ])
                                            ])
                                        ])
                                    ])
                                ])
                            ])
                        ])
                    ])
                ])
            ]
        };
    }
}
