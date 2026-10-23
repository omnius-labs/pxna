# Implementation Plans in Progress

This document covers work in progress and the dependencies between pieces of work.
Child documents cover the content of each piece of work, and for completed work, the code and git history are authoritative.

## 1. About This Document

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Plans in progress and dependencies |
| [design.md](./design.md#11-division-of-responsibility-between-documents) | Design decisions, invariants, deferred items |
| [terms.md](./terms.md#11-division-of-responsibility-between-documents) | Vocabulary |
| [issues.md](./issues.md) | Confirmed defects |

Create a child and add a row to the list before starting.
During implementation, advance the state of the child and of this document, and when a plan is wrapped up, delete the child and the row.

## 2. Plan List

| Plan | Purpose | State | Predecessor |
| --- | --- | --- | --- |
| [lan-plaintext-exposure](./plans/lan-plaintext-exposure.md) | A phone on the LAN or a VPN can log in to the compose setup over plain HTTP | Not started | - |
| [representative-thumbnail-refresh](./plans/representative-thumbnail-refresh.md) | Folder tiles show the new representative right after our own changes | Not started | - |
| [viewer-touch-gestures](./plans/viewer-touch-gestures.md) | The image viewer can be operated with one hand on a phone | Not started | - |
| [name-index-store](./plans/name-index-store.md) | Every non-hidden Entry of a Library is held in a name index that follows our own changes | Not started | - |
| [name-index-search](./plans/name-index-search.md) | A search from the root of a 10-million-Entry Library returns its first page within a second | Not started | name-index-store |

The two name index plans run in order. The other three are independent of each other and of them.
