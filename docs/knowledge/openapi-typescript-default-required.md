# openapi-typescript Makes a Property with a `default` Required in the Generated Types

**Source: Verified** (openapi-typescript 7.13.0 / 2026-11-28; a minimal spec / 2026-11-28)

One item of [knowledge.md](../knowledge.md).

## Fact

openapi-typescript 7.13.0 generates a property that has a `default` in the schema as a required property, even when the schema does not list it in `required`, and even in a request body.
A client that builds the request with the generated type then has to send a value for it.
The option `--default-non-nullable false` turns this off for the whole output.

## Basis

On 2026-11-28, a request schema with two optional booleans, `withDefault` with `default: false` and `withoutDefault` without one, was generated with openapi-typescript 7.13.0 without options.
The output had `withDefault: boolean;` and `withoutDefault?: boolean;`.
`openapi-typescript --help` of the same version lists `--default-non-nullable`, "Set to `false` to ignore default values when generating non-nullable types".

## Implication

The schemas of a listing request in `daemon/openapi.yaml` state their defaults in the description instead of `default:`, so an omitted option stays optional in the web client's types, while `createParents` of `CreateFolderRequest` keeps `default: false` and is required there ([design.md §4.2](../design.md#42-openapi-driven-code-generation)).
