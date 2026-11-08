# Design of the Continuous Integration

## 1. About This Document

This document covers when the checks run, which checks run for which change, which operating systems run them, and which result gates a merge.
The intended readers are developers who change `.github/workflows/` and AI agents that carry the decision forward.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are in that document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Triggers, change scope, jobs, the platform split, the merge gate, and the runtime conventions of the workflows |
| [design.md](../design.md) | Pxna's overall scope, overall structure, boundaries between concerns, and cross-cutting design decisions |
| [storage.md](./storage.md) | The operating systems the daemon targets and the storage behavior they must provide |
| [client.md](./client.md) | The client's build and its test tools |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | REST API paths, schemas, and status codes |
| [README.md](../../README.md) | How to run the same checks by hand |
| [Makefile.toml](../../daemon/Makefile.toml) | The Rust lint and test commands that the workflows call |
| [.github/workflows](../../.github/workflows) | The implementation of this design |

This document names what each job runs but does not define the commands, which live in the files above and in the client's scripts.
It holds no tool version and no history of a failed run.

### 1.2 Tense of This Document

The body describes the completed design in the present tense.
The implementation status and the remaining work are gathered in §9.
If you want to know what works now, read §9 first.

## 2. Responsibilities and Boundaries

The continuous integration answers one question for a change: may it merge?
It runs commands the repository already defines and holds no build logic of its own, so a failure in the workflow can be reproduced by running the same command on a developer's machine, apart from the operating system and the tool versions.
It does not release, deploy, or publish, and it holds no secret.
The container image and its distribution ([design.md §9.3](../design.md#93-distribution)) are a separate release concern and are not part of these workflows.

**A required check stops a change only before the merge.**
A check that runs after the merge can report a failure but cannot keep the change out of `main`, so the checks that must hold for every merge run on the pull request.

The dependency audit is outside this boundary.
It runs on a schedule and on dependency changes, and it does not gate a merge (§4.4).

## 3. Triggers and Change Scope

### 3.1 Triggers

One workflow, `ci`, runs on a pull request, on a push to `main` or `release/*`, and on a manual dispatch.
A pull request run tests the result of merging the branch into its base, which a push to the branch does not.
A push to `main` or `release/*` runs the merged state again on the operating systems of §5.2, and a manual dispatch runs the ref it is given in the same way.
The dependency audit is a separate workflow, `dependency-audit` (§4.4).

### 3.2 Change Scope

A `changes` job lists the files the run covers with `git diff --name-only` and decides which scopes the change touches.
For a pull request the list is the difference from the base, and for a push it is the difference from the previous head.
A manual dispatch and a change under `.github/` select every scope.

| Scope | Paths | Jobs |
| --- | --- | --- |
| daemon | `daemon/**` | `daemon-lint`, `daemon-test` |
| ui | `ui-web/**`, `daemon/openapi.yaml`, `daemon/sockets/image.schema.json`, `mise.toml`, `mise.lock` | `ui` |
| interface | `daemon/openapi.yaml`, `daemon/entrypoints/interface/**`, `daemon/entrypoints/gen-interface.sh` | `interface` |

A path can belong to more than one scope: `daemon/openapi.yaml` is in the daemon, ui, and interface scopes, and `daemon/sockets/image.schema.json` is in the daemon and ui scopes.
A change that touches no scope, such as a documentation-only change, runs `changes` and `gate` only.

## 4. Jobs

| Job | Runs when | Operating system | What it checks |
| --- | --- | --- | --- |
| `changes` | always | Linux | Decides the scopes of §3.2 |
| `daemon-lint` | daemon | Linux | `cargo make lint` |
| `daemon-test` | daemon | §5 | The daemon tests with all features |
| `ui` | ui | Linux | The client's lint, typecheck, build, unit tests, and regenerated API and socket types |
| `interface` | interface | Linux | The generated Rust interface equals the regeneration of `openapi.yaml` |
| `gate` | always | Linux | §6 |

### 4.1 Daemon

`daemon-test` calls `cargo nextest` directly and does not go through `cargo make test`, because that task depends on the lint and `daemon-lint` already runs it.
The run uses the `ci` nextest profile, which does not stop at the first failure, so one run lists every failing test ([nextest.toml](../../daemon/.config/nextest.toml)).
The matrix does not cancel the other operating systems when one fails.

The tests need ffmpeg 9 and, on Linux, `heif-dec` from libheif.
The workflow installs them per operating system and sets `PXNA_REQUIRE_FFMPEG` and `PXNA_REQUIRE_HEIF_DEC` so a missing binary fails the run instead of skipping the test.

### 4.2 Client

The `ui` job installs the toolchain with mise and runs the lint, typecheck, test, and build scripts of [package.json](../../ui-web/package.json).
It then runs `gen:api` and `gen:image-socket`, and fails when either changes a committed file under `ui-web/src/api` ([client.md §5.1](./client.md#generate-the-web-clients-api-types-from-openapiyaml)).
The build does not regenerate the types, so a contract change is visible in the diff and the build does not need the generator.

### 4.3 Generated Interface

The `interface` job runs `gen-interface.sh` and fails when the regenerated tree differs.
The script removes the generator's build timestamp, so an unchanged contract leaves no diff.
The script needs Java and downloads the generator, so a failed download fails the job, and the remedy is to run it again.

### 4.4 Dependency Audit

`dependency-audit` is a separate workflow.
It runs on a weekly schedule and on a change to a dependency manifest or lockfile, and it does not report to `gate`.
It runs `npm audit` for the client and `cargo audit` for the daemon, denying unmaintained and unsound advisories.
A finding is fixed in the dependency or filed in [issues.md](../issues.md); it does not block an unrelated merge.

## 5. Platforms

### 5.1 Pull Requests

A pull request runs the daemon jobs on Linux only.
The wait stays short, and the platform-specific code is checked after the merge.

### 5.2 Main and Release Branches

A push to `main` or `release/*`, and a manual dispatch, run `daemon-test` on Linux, macOS, and Windows.
A result of this run does not hold a change back, so a failure on macOS or Windows is the signal to fix the next change.

The test binary `omnius-core-migration::postgres` starts a Postgres container and needs a container runtime.
It is excluded on Windows and macOS, whose runners cannot start the container.
The remaining platform-specific setup is ffmpeg 9 on every runner and libheif on Linux and macOS.

## 6. The Merge Gate

`gate` always runs, needs every other job, and passes when each of them succeeded or was skipped by the scope.
It fails when any of them failed or was cancelled.
It is the only status check that branch protection should require on `main`, so adding or removing a job does not change the protection.

A skipped job does not report, and a native `paths:` filter on a workflow skips the whole workflow, so a required check cannot be a job that the scope may skip.
`gate` reports in every case, which is why the protection should name it alone.

## 7. Runtime Conventions

| Item | Convention |
| --- | --- |
| Concurrency | One group per workflow and ref. A new pull request run cancels the previous one; a run on `main` or `release/*` is never cancelled |
| Token | The workflow grants the read-only `contents` permission, and no job asks for more |
| Time limits | Every job sets `timeout-minutes`: 30 for the daemon jobs, 20 for `ui` and `interface`, and 5 for `changes` and `gate` |
| Rust cache | Keys scoped by operating system and job, saved only on `main`, so pull requests read the cache of `main` and do not crowd the cache quota |
| Node cache | The npm cache keyed by the lockfile and the mise tool cache |
| Setup steps | The free-space step, the submodule checkout, the timestamp restore, LLVM 20.1, cargo-make, and nextest stay as the current workflows have them. The client jobs install Node and the client's OpenAPI type generator with mise; the interface job installs Node with mise and downloads the Rust generator with `npx` |
| Actions | Pinned to a version tag and updated weekly by Dependabot, which also covers the Rust crates and the client's npm packages |

## 8. Design Decisions

### 8.1 Decided

#### Gate merges on a single required check

**Decision**
Branch protection on `main` requires the `gate` check and nothing else.

**Reason**
One check that reports in every case lets the jobs change without editing the protection, and §6 explains why no other job can be the required one.
The cost is that a broken `gate` stops every merge; an administrator removes the requirement to recover.

**Rejected alternatives**
Leaving the checks advisory keeps every merge possible, and a failing check can then merge unnoticed.

#### Run on pull requests and on pushes to the main branches

**Decision**
The workflow runs on `pull_request`, on a push to `main` or `release/*`, and on a manual dispatch. A push to another branch runs nothing.

**Reason**
A pull request run tests the merged result, and the run on the main branch tests it again on every platform.

**Rejected alternatives**
Running on a push to every branch shows a result before a pull request exists, but it tests the branch head rather than the merge, and a branch with a pull request runs twice.

#### Compute the change scope inside one workflow

**Decision**
One workflow decides the scopes of §3.2 in a `changes` job, and the later jobs use its answer.

**Reason**
The scope has to reach a check that always reports (§6), and only a job inside the workflow can skip itself and still let `gate` report.
The cost is a small script that the repository owns.

**Rejected alternatives**
Separate workflows with a native `paths:` filter are simpler, but a skipped workflow never reports, so no check of it could be required.

#### Test the other operating systems after the merge

**Decision**
A pull request runs Linux only, and a push to `main` or `release/*` runs Linux, macOS, and Windows.

**Reason**
The pull request wait stays short, and a platform-specific regression is found within one merge by the run on `main`.
The cost is that such a regression reaches `main` before it is seen.

**Rejected alternatives**
Running all three on every pull request catches the regression before the merge, but it multiplies the wait for every change.

#### Check the generated interface by regenerating it

**Decision**
The `interface` job regenerates the interface from `openapi.yaml` and fails if the tree differs.

**Reason**
Regeneration from scratch is a precondition of the contract, and a hand edit of a generated file is hard to see in a review.
The committed interface carries no build timestamp, so regenerating the current tree leaves it unchanged and a plain comparison holds.
The cost is the generator download, which can fail for a network reason.

**Rejected alternatives**
Leaving it to the review costs nothing, but nothing then enforces the precondition.

#### Run the dependency audit outside the merge gate

**Decision**
The dependency audit is a separate workflow that runs weekly and on dependency changes, and it is not a required check.

**Reason**
An advisory can appear with no change in the repository, so the audit needs a schedule, and a schedule cannot hold an unrelated merge.
The merge gate stays limited to checks the change can fix.

**Rejected alternatives**
Running the audit in `ci` gives it the `gate` treatment, but a newly published advisory would then block every merge until someone updates a dependency.

#### Pin the client's toolchain with mise

**Decision**
Node and `openapi-typescript` are installed by mise from `mise.toml` and `mise.lock`, and the client and interface jobs use `jdx/mise-action`.

**Reason**
The generator peers on an older TypeScript, so keeping it outside `package.json` lets the client track the current TypeScript while the generator stays pinned.
One lockfile pins both tools for local development and CI.

**Rejected alternatives**
Keeping the generator in `devDependencies` forces TypeScript to the generator's peer range.

### 8.2 Deferred

#### Run all three operating systems on every pull request

**Current state**
Only Linux runs before the merge (§5.1).

**Why not decide now**
The `main` run shows the platform failures within one merge, and the wait of a pull request is the cost of deciding earlier.

**Condition for deciding**
Decide when a failure that only the macOS or Windows run on `main` shows occurs a second time.

#### Run the merge checks on a schedule

**Current state**
The merge checks run only on a pull request, a main branch, or a manual dispatch. The dependency audit is the only check with a schedule (§4.4).

**Why not decide now**
No break from a toolchain or dependency update has been seen to hide until the next change.

**Condition for deciding**
Decide when an unrelated pull request is the first to show a break caused by a toolchain or dependency update.

## 9. Current State and Remaining Work

The `ci` workflow implements the jobs, the scope, and `gate`, and it replaced the earlier `test-daemon-all`, `test-daemon-slim`, and `test-web` workflows.
`main` must require `gate` in its branch protection, which is a repository setting outside the workflow; until it does, a failing check does not stop a merge.
The separate `dependency-audit` workflow runs weekly and on dependency changes.
The macOS and Windows runs may still show failures that Linux does not, and each such failure becomes an item in [issues.md](../issues.md).

For confirmed defects, see [issues.md](../issues.md).
