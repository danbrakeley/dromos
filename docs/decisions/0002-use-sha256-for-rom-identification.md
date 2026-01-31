---
status: accepted
date: 2026-01-31
---

# Use SHA-256 for ROM Identification

## Context and Problem Statement

Dromos needs to uniquely identify ROM files to build a graph of binary diffs.

Which cryptographic hash algorithm should be used for ROM identification?

## Considered Options

- SHA-256 (SHA-2 family)
- SHA-512 (SHA-2 family)
- SHA3-256 (SHA-3 family)

## Decision Outcome

Chosen option: "SHA-256", because it provides strong collision resistance, is the de facto standard for ROM databases, and offers the best balance of security, performance, and compatibility.

## More Information

SHA-256 can be supplemented with CRC32 for quick validation (many ROM databases include both), but SHA-256 serves as the primary identifier.

NOTE: This does not solve the problem of the same underlying ROM having different headers. At some point, we'll need to recognize headers, and ignore fields in the header that may be non-deterministic or not relevant.
