# receipt_history

Reserved; the library lands here. Execution receipts over time (what ran, when, on which spec and code digest, with what outcome) as a stored history with its checks — one kind of history in the differences ecosystem.

This repository holds the 0.0.1 placeholder source as published on crates.io
([`receipt_history`](https://crates.io/crates/receipt_history)): it holds the name, and nothing in it is usable yet.

## Ownership

Developed and published by James Hunt. The repository is held by the 1o organization
(https://github.com/1o-llc). Assignment of the copyright to 1o is pending documentation. The license texts
(LICENSE-MIT, LICENSE-APACHE) are the ones the other 1o crates carry.

## Releasing

Bump `version` in Cargo.toml, add a dated `## <version> (YYYY-MM-DD)` entry to CHANGELOG.md, and push to
`main`. CI (.github/workflows/ci.yml) runs the tests and clippy, then publishes that version to crates.io.

## License

MIT OR Apache-2.0.
