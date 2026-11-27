# axum's `Json` Extractor Rejects a Body That Is Not JSON with 400 and One That Does Not Fit the Type with 422

**Source: Verified** (axum 0.8.9 / 2026-11-26; the source of `axum::extract::rejection` in axum 0.8.9 and a route test of the daemon / 2026-11-26)

One item of [knowledge.md](../knowledge.md).

## Fact

When the `Json` extractor of axum 0.8 cannot read a request body, it answers before the handler runs, with a plain-text body and a status that depends on the failure.
A body that is not syntactically valid JSON is 400 (`JsonSyntaxError`).
A body that is valid JSON but does not deserialize into the target type, such as one with an unknown value of an enum, a missing required property, or a value of another type, is 422 (`JsonDataError`).
A request without `Content-Type: application/json` is 415 (`MissingJsonContentType`).

## Basis

`src/extract/rejection.rs` of axum 0.8.9 defines `JsonDataError` with `#[status = UNPROCESSABLE_ENTITY]` and the body "Failed to deserialize the JSON body into the target type", `JsonSyntaxError` with `#[status = BAD_REQUEST]` and "Failed to parse the request body as JSON", and `MissingJsonContentType` with `#[status = UNSUPPORTED_MEDIA_TYPE]`.
The axum documentation of `Json` lists the same three failures, without their statuses.

On 2026-11-26, a route test of the daemon sent `PUT /api/v1/preferences` with `archiveOpen` set to a value its enum does not have, and received 422.
The handlers of the generated router take the body with `Json`, as `put_preferences` in `daemon/entrypoints/interface/src/server/mod.rs` does.

## Implication

The generated router rejects a body that does not fit the types of the contract with 422 and a body with a value outside a range of the contract with 400, before the handler ([design.md §4.4](../design.md#44-error-model-and-http-mapping)).
