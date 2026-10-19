// Copyright © 2026 1o. All rights reserved.

//! `receipt_history`: execution receipts over time.
//!
//! A run leaves one [`Receipt`]: what ran (the line), when, on which spec and
//! code digest, with what decision and outputs. The [`Journal`] keeps every
//! receipt in one SQLite file (`runs`, with the `jobs` and `tests` tables
//! beside it) and each one as `receipts/<seq>.json`. Its sequence is
//! uh-tom-mick's checked counter, brought up to the highest receipt inside
//! the writing transaction, so two writers on one file never take one number.
//! The journal is machine state; the receipt files are the record, and
//! [`Journal::rebuild_from`] puts back every receipt a checkout carries.
//!
//! - [`receipt`]: the receipt, its schema and its runners.
//! - [`journal`]: the journal, its tables, keeping and reading receipts, the rebuild.
//! - [`digest`]: every digest a receipt carries, one way.
//! - [`files`]: files written whole.
//! - [`refusal`]: the refusals, as liaise codes.

pub mod digest;
pub mod files;
pub mod journal;
pub mod receipt;
pub mod refusal;

pub use files::write_whole;
pub use journal::{JobRow, Journal, Rebuilt, TestRow, JOURNAL_BUSY, JOURNAL_SCHEMA};
pub use receipt::{line_runner, Receipt, RECEIPT_SCHEMA, RUNNER_IN_PROCESS, RUNNER_LINE};
pub use refusal::{Error, Refusal, Result};
