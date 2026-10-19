# receipt_history

Execution receipts over time. A run leaves one receipt: what ran (the line), when, on which spec and code digest,
with what decision and outputs (`custodian.receipt.v1`). The journal keeps every receipt in one SQLite file
(`runs`, with the `jobs` and `tests` tables beside it) and each one as `receipts/<seq>.json`.

- The sequence is uh-tom-mick's checked counter, brought up to the highest receipt inside the writing transaction
  (`BEGIN IMMEDIATE`), so two journals on one file, in one process or two, never take one number.
- `Journal::keep` takes a number above every row and every receipt file already written, writes the file whole
  and inserts the row: a receipt file is never written over.
- The journal is machine state; the receipt files are the record. `Journal::rebuild_from` puts back every receipt
  file the journal has no row for, under its own number, and the sequence continues from the highest.
- A caller that keeps tables of its own in the same file adds them on `Journal::connection` and writes inside
  `Journal::writing` (custodian keeps its readings index, events, test lines, installs and checkpoints this way).

```rust,no_run
use receipt_history::Journal;
# fn demo(mut r: receipt_history::Receipt) -> receipt_history::Result<()> {
let j = Journal::open("state/journal.sqlite".as_ref())?;
let (seq, file) = j.keep(&mut r, |n| Ok(format!("data/receipts/{n}.json").into()))?;
let since = j.since(seq - 10)?;              // the receipts after a point
let back = j.rebuild_from("data/receipts".as_ref(), |_, _| Ok(()))?;
# Ok(()) }
```

`digest` holds every digest a receipt carries, one way (stable_digest's SHA-256 over a value's JSON with sorted
keys); `write_whole` writes a file whole (write-then-rename).

The journal came out of custodian_kit's `store` and `receipts` and custodian's `journal`; custodian_kit re-exports
it where it was (`custodian_kit::store::Journal`, `custodian_kit::receipts::Receipt`), and custodian keeps its own
tables on it.

## Refusals

`RCPT0001` the journal cannot be read or written; `RCPT0002` a file the journal keeps cannot be written.

## Releasing

Bump `version` in Cargo.toml, add a dated `## <version> (YYYY-MM-DD)` entry to CHANGELOG.md, and push to
`main`. CI (.github/workflows/ci.yml) runs the tests on stable and Rust 1.89 and clippy, then publishes that
version to crates.io through trusted publishing; a version already on crates.io publishes nothing.

## Ownership

Developed and published by James Hunt. The repository is held by the 1o organization
(https://github.com/1o-llc). Assignment of the copyright to 1o is pending documentation. The license texts
(LICENSE-MIT, LICENSE-APACHE) are left as they were found. Every file new in 0.1.0 carries the notice
Copyright © 2026 1o. All rights reserved.

## License

MIT OR Apache-2.0
