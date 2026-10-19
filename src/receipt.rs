// Copyright © 2026 1o. All rights reserved.

//! A receipt: one run as the journal keeps it and as `receipts/<seq>.json`
//! holds it (`custodian.receipt.v1`).

use serde::{Deserialize, Serialize};

/// One run, as the journal keeps it and as `receipts/<seq>.json` holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub schema: String,
    pub seq: i64,
    pub utc: String,
    /// What ran: `reading`, `test`, `check`, `git`, `gate`, `job-close`.
    pub command: String,
    /// The spec, the crate, the check.
    pub name: String,
    pub spec: Option<String>,
    pub spec_hash: Option<String>,
    /// klepto's fingerprint of the files the scan read.
    pub code_digest: Option<String>,
    /// The registry's polymorphic index key of the reading (`kidx_...`).
    pub index_key: Option<String>,
    pub plan_id: String,
    pub plan_digest: String,
    pub operation_digest: String,
    /// xccute's receipt digest over plan, operation and decision.
    pub receipt_digest: String,
    /// The line as it ran.
    pub line: String,
    pub exit_code: Option<i32>,
    pub accepted: bool,
    pub disposition: String,
    pub reason: String,
    pub outputs: Vec<String>,
    pub job: Option<String>,
    /// For a reuse: the run whose reading answered.
    pub reused_seq: Option<i64>,
    pub millis: i64,
    /// Which runner answered: `line` (the declared line, run through the
    /// executor) or `in_process` (the same line's reading answered in this
    /// process). A receipt for every run, whichever runner answered.
    #[serde(default = "line_runner")]
    pub runner: String,
}

/// The runner a receipt names when none is written (receipts before the field).
pub fn line_runner() -> String {
    RUNNER_LINE.to_string()
}

pub const RUNNER_LINE: &str = "line";
pub const RUNNER_IN_PROCESS: &str = "in_process";

pub const RECEIPT_SCHEMA: &str = "custodian.receipt.v1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_receipt_written_before_the_runner_field_reads_as_a_line_run() {
        let json = r#"{"schema":"custodian.receipt.v1","seq":4,"utc":"2026-10-19T07:38:03Z","command":"reading","name":"env","spec":null,"spec_hash":"s","code_digest":"c","index_key":null,"plan_id":"p","plan_digest":"pd","operation_digest":"od","receipt_digest":"rd","line":"custodian exec-reading","exit_code":0,"accepted":true,"disposition":"Continue","reason":"read","outputs":["readings/env/x.md"],"job":null,"reused_seq":null,"millis":12}"#;
        let r: Receipt = serde_json::from_str(json).unwrap();
        assert_eq!((r.seq, r.runner.as_str()), (4, RUNNER_LINE));
    }
}
