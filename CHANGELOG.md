# Changelog

## 0.1.0 (2026-10-19)

The receipts journal, moved out of custodian_kit and custodian into a crate of
its own. What ran, when, over which spec and code, and with what outcome was
kept in two places: custodian_kit's journal recorded receipts, and custodian's
journal kept them again with its own sequencing and rebuilt the journal from
the receipt files. Both now open this one journal. custodian_kit re-exports
everything it exported before at the same paths, and custodian keeps its own
tables on this journal's connection. Every link is pinned to 443d8a8.

- The receipt, its schema and its runners: `Receipt`, `RECEIPT_SCHEMA`,
  `RUNNER_LINE`, `RUNNER_IN_PROCESS`, `line_runner`
  ([src/receipt.rs#L10](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/receipt.rs#L10)). Moved from [custodian_kit src/receipts.rs#L15](https://github.com/1o-llc/custodian_kit/blob/b17adf76375f72bebfb5ae1d1d93de3826369aac/src/receipts.rs#L15).
- The journal and its tables: `JOURNAL_SCHEMA` ([src/journal.rs#L26](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L26)),
  `Journal` ([src/journal.rs#L105](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L105)), `JobRow`, `TestRow`,
  `JOURNAL_BUSY`, and the readers `runs`, `runs_of_job`, `readings_of`,
  `same_search`, `rehome`, `put_job`, `job`, `put_test`, `tests` and
  `tests_of_runs`. Moved from [custodian_kit src/store.rs#L280](https://github.com/1o-llc/custodian_kit/blob/b17adf76375f72bebfb5ae1d1d93de3826369aac/src/store.rs#L280) and
  [custodian_kit src/store.rs#L291](https://github.com/1o-llc/custodian_kit/blob/b17adf76375f72bebfb5ae1d1d93de3826369aac/src/store.rs#L291). The SQL is unchanged.
- One sequencing rule. `Journal::keep` ([src/journal.rs#L262](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L262)) takes the
  next number inside one writing transaction (`BEGIN IMMEDIATE`), above every
  row and every receipt file already written, writes the file whole and
  inserts the row. Moved from [custodian src/journal.rs#L274](https://github.com/1o-llc/custodian/blob/cd2281740ae5a79290311217c9238509dcc8cf1f/src/journal.rs#L274). `Journal::record`
  ([src/journal.rs#L248](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L248)) now takes its number inside the same kind of
  transaction, above every row. Before, it took the counter's next number as
  seeded when the journal was opened ([custodian_kit src/store.rs#L377](https://github.com/1o-llc/custodian_kit/blob/b17adf76375f72bebfb5ae1d1d93de3826369aac/src/store.rs#L377)), so a second
  journal on the same file could insert a number that was already taken.
  Proven by `two_journals_on_one_file_never_take_one_number`
  ([tests/journal.rs#L45](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/tests/journal.rs#L45)), moved from [custodian src/journal.rs#L718](https://github.com/1o-llc/custodian/blob/cd2281740ae5a79290311217c9238509dcc8cf1f/src/journal.rs#L718) and extended
  to cover `record` on both journals; and by
  `a_receipt_file_is_never_written_over` ([tests/journal.rs#L66](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/tests/journal.rs#L66)).
- The rebuild. `Journal::rebuild_from` ([src/journal.rs#L382](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L382)) puts back
  every receipt file the journal has no row for, under its own number, inside
  one writing transaction, and the sequence continues from the highest. A
  caller's own index of each run goes in through the `also` hook in the same
  transaction; custodian indexes its readings that way. Moved from
  [custodian src/journal.rs#L534](https://github.com/1o-llc/custodian/blob/cd2281740ae5a79290311217c9238509dcc8cf1f/src/journal.rs#L534), where the runs were rebuilt together with
  custodian's own rows. Proven by
  `a_fresh_checkout_s_journal_is_rebuilt_from_its_receipt_files`
  ([tests/journal.rs#L104](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/tests/journal.rs#L104)).
- New: `Journal::connection` ([src/journal.rs#L215](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L215)) and
  `Journal::writing` ([src/journal.rs#L220](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L220)), for a caller that keeps
  tables of its own in the same file; `Journal::since`
  ([src/journal.rs#L292](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/journal.rs#L292)) returns the receipts after a given number;
  `Journal::count` returns how many receipts the journal holds.
- The digests a receipt carries: `digest::of`, `tagged`, `canonical`,
  `of_value` and `short` ([src/digest.rs#L32](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/digest.rs#L32)), with the test
  `canonical_forms_do_not_depend_on_field_order` ([src/digest.rs#L47](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/digest.rs#L47)).
  Moved from [custodian_kit src/digest.rs#L30](https://github.com/1o-llc/custodian_kit/blob/b17adf76375f72bebfb5ae1d1d93de3826369aac/src/digest.rs#L30) with that test ([custodian_kit src/digest.rs#L45](https://github.com/1o-llc/custodian_kit/blob/b17adf76375f72bebfb5ae1d1d93de3826369aac/src/digest.rs#L45)).
- `write_whole` ([src/files.rs#L15](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/src/files.rs#L15)) writes a file whole
  (write-then-rename). Moved from [custodian_kit src/store.rs#L119](https://github.com/1o-llc/custodian_kit/blob/b17adf76375f72bebfb5ae1d1d93de3826369aac/src/store.rs#L119). A receipt file now
  ends with a newline, as custodian_kit's always did; custodian's did not.

Proof: `cargo test`, 7 tests: 2 unit tests, 5 journal tests in
tests/journal.rs. The journal tests include
`a_journal_from_before_the_runner_field_gains_its_column`
([tests/journal.rs#L128](https://github.com/1o-llc/receipt_history/blob/443d8a8be567dfcd30ffe46577f2c91582640873/tests/journal.rs#L128)). custodian_kit's 213 tests and custodian's tests
pass against this crate.
