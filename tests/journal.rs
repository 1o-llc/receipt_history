// Copyright © 2026 1o. All rights reserved.

//! The journal: numbers never taken twice, receipts kept with their files, a
//! journal rebuilt from the files a checkout carries.

use receipt_history::{JobRow, Journal, Receipt, TestRow, RECEIPT_SCHEMA, RUNNER_IN_PROCESS};
use std::path::PathBuf;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("receipt-history-{name}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn receipt(name: &str) -> Receipt {
    Receipt {
        schema: RECEIPT_SCHEMA.into(),
        seq: 0,
        utc: "2026-10-19T12:00:00Z".into(),
        command: "reading".into(),
        name: name.into(),
        spec: None,
        spec_hash: Some("s".into()),
        code_digest: Some("c".into()),
        index_key: Some("k".into()),
        plan_id: "custodian".into(),
        plan_digest: "p".into(),
        operation_digest: "o".into(),
        receipt_digest: "r".into(),
        line: "custodian run x".into(),
        exit_code: Some(0),
        accepted: true,
        disposition: "Continue".into(),
        reason: "done".into(),
        outputs: vec!["readings/x/a.json".into()],
        job: None,
        reused_seq: None,
        millis: 3,
        runner: RUNNER_IN_PROCESS.into(),
    }
}

/// Moved from custodian's journal (`two_journals_on_one_file_never_take_one_number`).
#[test]
fn two_journals_on_one_file_never_take_one_number() {
    let dir = dir("two");
    let file = dir.join("journal.sqlite");
    let a = Journal::open(&file).unwrap();
    let b = Journal::open(&file).unwrap();
    let at = |n: i64| Ok(dir.join(format!("{n}.json")));
    let (n1, _) = a.keep(&mut receipt("one"), at).unwrap();
    let (n2, _) = b.keep(&mut receipt("two"), at).unwrap();
    let (n3, f3) = a.keep(&mut receipt("three"), at).unwrap();
    assert_eq!((n1, n2, n3), (1, 2, 3));
    let back: Receipt = serde_json::from_str(&std::fs::read_to_string(f3).unwrap()).unwrap();
    assert_eq!((back.seq, back.name.as_str()), (3, "three"));
    assert_eq!(a.receipt(2).unwrap().unwrap().name, "two");
    assert_eq!(b.runs().unwrap().len(), 3);
    // a record without a file takes the next number too, whichever journal holds the counter
    let r = b.record(receipt("four")).unwrap();
    let r5 = a.record(receipt("five")).unwrap();
    assert_eq!((r.seq, r5.seq), (4, 5));
}

#[test]
fn a_receipt_file_is_never_written_over() {
    let dir = dir("files");
    std::fs::write(dir.join("7.json"), "{}").unwrap();
    let j = Journal::open(&dir.join("journal.sqlite")).unwrap();
    let (n, f) = j.keep(&mut receipt("one"), |n| Ok(dir.join(format!("{n}.json")))).unwrap();
    assert_eq!(n, 8);
    assert!(f.ends_with("8.json"));
    assert_eq!(std::fs::read_to_string(dir.join("7.json")).unwrap(), "{}");
}

#[test]
fn readings_reuses_jobs_tests_and_since() {
    let dir = dir("rows");
    let j = Journal::open(&dir.join("journal.sqlite")).unwrap();
    let first = j.record(receipt("env")).unwrap();
    let mut reuse = receipt("env");
    reuse.reused_seq = Some(first.seq);
    j.record(reuse).unwrap();
    let mut other = receipt("env");
    other.code_digest = Some("c2".into());
    other.job = Some("j-1".into());
    let third = j.record(other).unwrap();
    assert_eq!(j.readings_of("env").unwrap().iter().map(|r| r.seq).collect::<Vec<_>>(), [1, 3]);
    assert_eq!(j.same_search("s", "c").unwrap().unwrap().seq, 1);
    assert_eq!(j.same_search("s", "c2").unwrap().unwrap().seq, 3);
    assert_eq!(j.runs_of_job("j-1").unwrap().len(), 1);
    assert_eq!(j.since(1).unwrap().iter().map(|r| r.seq).collect::<Vec<_>>(), [2, 3]);
    assert_eq!(j.count().unwrap(), 3);
    j.put_job(&JobRow { id: "j-1".into(), name: "batch".into(), kind: "specs".into(), state: "running".into(), folder: "f".into(), opened_utc: Some("2026-10-19T12:00:00Z".into()), closed_utc: None }).unwrap();
    assert_eq!(j.job("batch").unwrap().unwrap().id, "j-1");
    j.put_test(&TestRow { seq: third.seq, package: "p".into(), target: "lib".into(), test: "t".into(), outcome: "ok".into(), millis: Some(4) }).unwrap();
    assert_eq!(j.tests_of_runs(&[3]).unwrap().len(), 1);
    let moved = j.rehome("readings/", "state/jobs/done/j-1/readings/").unwrap();
    assert_eq!(moved.len(), 3);
    assert_eq!(j.receipt(1).unwrap().unwrap().outputs, ["state/jobs/done/j-1/readings/x/a.json"]);
}

#[test]
fn a_fresh_checkout_s_journal_is_rebuilt_from_its_receipt_files() {
    let dir = dir("rebuild");
    let receipts = dir.join("receipts");
    let a = Journal::open(&dir.join("first.sqlite")).unwrap();
    for name in ["one", "two", "three"] {
        a.keep(&mut receipt(name), |n| Ok(receipts.join(format!("{n}.json")))).unwrap();
    }
    // the journal is machine state: a checkout carries only the receipt files
    let b = Journal::open(&dir.join("fresh.sqlite")).unwrap();
    let mut seen = Vec::new();
    let back = b.rebuild_from(&receipts, |_, r| {
        seen.push(r.name.clone());
        Ok(())
    })
    .unwrap();
    assert_eq!((back.receipts, back.last), (3, 3));
    assert_eq!(seen, ["one", "two", "three"]);
    assert_eq!(b.runs().unwrap(), a.runs().unwrap());
    // the sequence continues from the highest receipt; a second rebuild puts back nothing
    assert_eq!(b.record(receipt("four")).unwrap().seq, 4);
    assert_eq!(b.rebuild_from(&receipts, |_, _| Ok(())).unwrap().receipts, 0);
}

#[test]
fn a_journal_from_before_the_runner_field_gains_its_column() {
    let dir = dir("old");
    let file = dir.join("journal.sqlite");
    let old = runs_without_runner();
    let c = Journal::open(&file).unwrap();
    c.connection().execute_batch("DROP TABLE runs;").unwrap();
    c.connection().execute_batch(&old).unwrap();
    c.connection().execute("INSERT INTO runs (utc, command, name, plan_id, plan_digest, operation_digest, receipt_digest, line, accepted, disposition, reason, outputs, millis) VALUES ('u', 'reading', 'env', 'p', 'pd', 'od', 'rd', 'l', 1, 'Continue', 'r', '[]', 1)", []).unwrap();
    drop(c);
    let j = Journal::open(&file).unwrap();
    assert_eq!(j.receipt(1).unwrap().unwrap().runner, "line");
}

/// The `runs` table as it was before the runner column.
fn runs_without_runner() -> String {
    receipt_history::JOURNAL_SCHEMA.split("CREATE INDEX").next().unwrap().replace(",\n  runner TEXT NOT NULL DEFAULT 'line'", "")
}
