# Vendored optimization-pattern files

The `pbo-patterns*.txt` files in this directory are copied verbatim from
[mdlz80optimizer](https://github.com/santiontanon/mdlz80optimizer) by Santiago
Ontañón, licensed under the Apache License 2.0 (see `LICENSE` alongside them).

They are compiled into this crate as its built-in rule set (see
`builtin_rules.rs`), and are also the corpus the pattern parser is validated
against, so a grammar regression that would break real upstream files fails CI
rather than being discovered later. The matching engine only *executes* the
subset of rules whose constraints it actually supports — parsing all of them is
deliberately independent of that.

Kept **verbatim**, deliberately: re-vendoring a newer upstream release should be
a plain file copy, with the tag/goal filtering and the supported-constraint
subset doing the selection at load time, rather than a hand-edited fork that
drifts away from upstream.

Upstream sources:

- `pbo-patterns.txt` — the base pattern library
- `pbo-patterns-size.txt` — size-oriented patterns (`include`s the base file)
- `pbo-patterns-speed.txt` — speed-oriented patterns (`include`s the base file)

Note that the size and speed files contain **directly opposing** rules (size
turns `jp` into `jr`; speed turns `jr` into `jp`), which is why they are exposed
as alternative goals rather than unioned into one set.
