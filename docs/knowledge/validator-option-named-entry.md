# validator 0.20 Cannot Derive `Validate` for a Nested Optional Field Named `entry`

**Source: Verified** (validator 0.20.0, validator_derive 0.20.1, openapi-generator 7.23.0 rust-axum, rustc 1.99.0 / 2026-11-28; the source of validator_derive 0.20.1 and a minimal generated crate / 2026-11-28)

One item of [knowledge.md](../knowledge.md).

## Fact

A struct that derives `validator::Validate` does not compile when it has a field named `entry` of type `Option<T>` marked `#[validate(nested)]`.
The compiler reports that the method `validate` cannot be called on a `std::collections::hash_map::VacantEntry`.
The same field with another name compiles, and so does a required field named `entry`.
openapi-generator's `rust-axum` generator writes exactly such a field for an optional property named `entry` whose type is an object schema, so the generated crate does not compile.

## Basis

In validator_derive 0.20.1, `if_let_option_wrapper` in `src/types.rs` wraps the validation of an `Option` field in `if let Some(ref <field name>) = self.<field name>`, so inside it the field's own name is the binding.
`nested_tokens` in `src/tokens/nested.rs` then writes `if let std::collections::hash_map::Entry::Vacant(entry) = errors.0.entry(...) { errors.merge_self(..., (&<field name>).validate()); }`, whose `Vacant(entry)` shadows a binding named `entry`.

On 2026-11-28, a spec with one object schema `Wrapper` having two optional properties `entry` and `other` of the same object type was generated with openapi-generator 7.23.0 (`rust-axum`) and checked with `cargo check`.
It failed with one error, `E0599` on the derive of `Wrapper`, naming `VacantEntry<'_, Cow<'static, str>, ValidationErrorsKind>`; `other` produced no error.
The generated `EntryResponse` of `daemon/openapi.yaml`, whose required `entry` is marked `#[validate(nested)]`, compiles.

## Implication

An optional property of an object type in `daemon/openapi.yaml` is not named `entry`; the Entry of a metadata result is `detail` for this reason ([design.md §4.2](../design.md#42-openapi-driven-code-generation)).
