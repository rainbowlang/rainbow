# design of Rainbow

Rainbow is a language designed first & foremost for embedding in other languages. It's also intended to be edited primarily through yet-to-be-developed structural editor. This document attempts to explain what constraints were chosen, why those constraints are valuable, and how Rainbow attempts to provide something useful within them.

(Later, it also talks about some boring details like syntax & typing rules).

## What Rainbow is for

Rainbow is meant for embedding programs written by non-expert programmers inside of larger programs written by somwehat-more-expert programmers. Think: core "business logic" inside a networked service.

## Constraints

### well-typed Rainbow progams should never crash (unexpectedly)

Given that the archetypical Rainbow programmer might never have programmed before, _and_ we want archetypical "expert programmers" to allow Rainbow programs to run in-process, safety is priority #1. There must be **no** undefined runtime behaviour, segfaults, stack-overflows etc.

In practice this means that Rainbow is fully statically typed (though the programs themselves do not contain any type annotations). This prevents type errors arising from nonsensical operations such as accessing a record field on a numeric value.

A more interesting aspect of the type system is that it reflects the concept of "partial functions": functions that may fail to produce a valid output for all possible inputs. The Rainbow type checker requires that every call to such a function is wrapped in a `try: {} or: {}` construct, such that every program must specify what to do in case of a failure.

Example partial functions:

- division (division by zero is undefined).
- lookups in a map (no way to ensure the key may be present).
- any function that performs I/O.

### well-typed Rainbow programs should always terminate

Here are two ways to write a program that doesn't terminate, demonstrated in JavaScript.

Loops:

```js
while (true) {}
```

Recursion:

```js
function go() {
  return go();
}

go();
```

_(technically the recursion example will terminate due to running out of stack-space, but there are languages in which that would not be the case)._

In Rainbow we avoid both of these.

Looping constructs are easy to avoid, simply don't add them to the language. Instead use iteration functions (e.g. `map`, `reduce` and friends). There's still a space here for causing problems by appending to a list while iterating over it, but Rainbow closes that loophole that by not providing mutation or rebinding of identifiers.

Unbounded recursion is a bit trickier to prevent in most languages, with different static analyses being possible with different trade-offs in language design (e.g. dependent types). Rainbow doesn't go this route, and instead simply offers no facilities for naming & jumping to a particular piece of code. There is no way to author Rainbow functions in Rainbow itself. This trade-off is considered acceptable for Rainbows intended use: there's a great number of small and useful programs that can be expressed clearly without these constructs.

## Syntax

### values

- numbers: `1`, `1.1`
- strings: `"neato"` `"I have \"quotes\" inside"`
- booleans: `true` and `false`
- lists: `[ 1 2 3 ]`
- records: `[ key = "value" ]`, `myrecord.key`
- function calls: `sum: [ 1 2 3 ]`, `countFrom: 1 to: 3`, `sum: countFrom: 1 to: 3`, `if: true then: false`
- blocks: `{ x => calc: x plus: 3 }`, `{ calc: 1 plus: 3 }`

### Function calls

A unique aspect of Rainbow syntax is that all function arguments are keyword arguments. The type of a function specifies for each keyword: the expected type, whether it is required, and whether it is variadic. This means a single function may have multiple variadic arguments. For example, `if`:

```
if: foo and: bar or: baz and: qux or: zub then: 12 else: 13
```

Unlike _selectors_ in Smalltalk/Objective-C/Swift, only the **first** keyword is used when dispatching the call, the rest are part of the functions type signature.

### Blocks

Rainbow does not provide facilities for defining functions in Rainbow. This prevents recursion (and accidentally non-terminating code). Instead, higher-order programming is achieved through "blocks".

Blocks act as a quoting mechanism for some Rainbow code + the ability to rebind an identifier (or set of identifiers) on each block execution. For example, `{ a b => calc: a add: b }` is a block taking two arguments, while `{ fetch: "http://rainbowlang.github.io" }` is a block taking no arguments.

There is no syntax to call or apply blocks: the block body can only be evaluated by functions defined in the host program. (This is again an intentional omission to ensure Rainbow programs can't accidentally express unbounded recursion).

#### Block coercion

No-argument blocks are the one case of implicit coercion in Rainbow. The coercion rules for blocks are simple and purely syntactic:

1. If a function argument expects a block of zero arguments and is given a value instead, the term will be automatically converted to the block `{ term }`, whose evaluation is controlled by the function implementation.
2. If a zero-argument block is provided in a context that does _not_ expect a block, the block body will be unwrapped (and eagerly evaluated). This allows zero-argument blocks to also be used like parentheses for delimiting expressions.

By rule #1 these are equivalent:

- `if: foo then: bar`
- `if: foo then: { bar }`

And by rule #2 all of these are equivalent:

- `sum: countFrom: 1 to: max: foo or: bar`
- `sum: { countFrom: 1 to: { max: foo or: bar } }`
- `sum: countFrom: { 1 } to: max: foo or: { bar }`

## Typing rules

### Primitives

Primitive types only satisfy themselves.

The primitive types are:

- `string`
- `number`
- `boolean`
- `time`

### Lists

Lists are homogeneously typed. A list type `Left` is satisfied by another list type `Right` iff the element type of `Left` is satisfied by the element type of `Right`.

The type of a list containing elements of type `E` is written `[ E ]`

### Records

Records contain a fixed set of field identifiers that map to types. A given field may be optional.

A record type `Left` is satisfied by another record type `Right` if every non-optional field in `Left` is present (and non-optional) in `Right` and has the same type.

The type of a record with a required field `foo` of type `F` and optional field `bar` of type `B` is written `[ foo = F bar = B? ]`.

### Blocks

Blocks are typed by a (possibly empty) list of input types and an output type. A block type `Left` is satisfied by another block type `Right` iff:

1. `Right` expects _at most_ as many inputs as `Left`.
2. Each input type of `Left` is satisfied by corresponding input type of `Right`.
3. The output type of `Left` is satisfied by the output type of `Right`.

The type of a block taking arguments of types `A` and `B` and returns a value of type `C` is written `{ A B => C }`.

The type of a block taking no arguments and returning type `T` is written `{ T }`

### Functions

Functions are typed by a set of identifiers mapping to input types and their output type. Any one of these identifiers may be variadic, and any but the first (the function name) may be optional. Because Rainbow can only call functions (it is not possible to define new functions in Rainbow, or pass functions as values), there is no concept of satisfiability for a function types.

_(The below notation is subject to change)_

However, it's still useful to be able to write down the type of a function for documentation. The type of a function named `foo` taking a `foo` argument of type `F`, a variadic number of arguments named `bar` of type `B`, an optional argument `baz` of type `Z`, and returning type `C` would therefore be written `foo:F [bar]:B baz:?Z => C`.

A more useful example is the type of `if`, which is written as follows:

```
{
    if: boolean
    [and]: { boolean }
    [or]: { boolean }
    then: { A }
    else: { A }
}
```

### Side-effects

The below is **wrong**, such a thing is still intended, but needs a more thorough review of the various approaches out there. _(I'll probably just ape what Frank does)_

Side effects are represented by an `Effect` type containing a list of "effect tags". Rainbow does not interpret any side-effects itself, it is up to the embedding program to enact whatever effects the Rainbow program produces by providing an effect interpreter (colloquially called an "effector").

TODO - better describe effect tags/categorization (e.g. distinction between reversible and permanent effects).
