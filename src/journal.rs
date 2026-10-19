// Copyright © 2026 1o. All rights reserved.

//! The journal: one SQLite file holding a receipt per run (`runs`), the jobs
//! and the test results, declared once in [`JOURNAL_SCHEMA`]. A caller that
//! keeps tables of its own in the same file adds them on
//! [`Journal::connection`].
//!
//! Its sequence is uh-tom-mick's checked counter, brought up to the highest
//! receipt inside the writing transaction (`BEGIN IMMEDIATE`), so two
//! journals on one file, in one process or two, never take one number. A
//! receipt kept with its file ([`Journal::keep`]) also takes a number above
//! every receipt file already written: a file is never written over.

use crate::files::write_whole;
use crate::receipt::{Receipt, RECEIPT_SCHEMA};
use crate::refusal::{Refusal, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uh_tom_mick::AtomicCounter;

/// How long a write to the journal waits for another writer: the one wait the journal has.
pub const JOURNAL_BUSY: std::time::Duration = std::time::Duration::from_secs(10);

/// The journal's tables, declared once.
pub const JOURNAL_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS runs (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  utc TEXT NOT NULL,
  command TEXT NOT NULL,
  name TEXT NOT NULL,
  spec TEXT,
  spec_hash TEXT,
  code_digest TEXT,
  index_key TEXT,
  plan_id TEXT NOT NULL,
  plan_digest TEXT NOT NULL,
  operation_digest TEXT NOT NULL,
  receipt_digest TEXT NOT NULL,
  line TEXT NOT NULL,
  exit_code INTEGER,
  accepted INTEGER NOT NULL,
  disposition TEXT NOT NULL,
  reason TEXT NOT NULL,
  outputs TEXT NOT NULL,
  job TEXT,
  reused_seq INTEGER,
  millis INTEGER NOT NULL,
  runner TEXT NOT NULL DEFAULT 'line'
);
CREATE INDEX IF NOT EXISTS runs_same_search ON runs (command, spec_hash, code_digest);
CREATE TABLE IF NOT EXISTS jobs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  state TEXT NOT NULL,
  folder TEXT NOT NULL,
  opened_utc TEXT,
  closed_utc TEXT
);
CREATE TABLE IF NOT EXISTS tests (
  seq INTEGER NOT NULL,
  package TEXT NOT NULL,
  target TEXT NOT NULL,
  test TEXT NOT NULL,
  outcome TEXT NOT NULL,
  millis INTEGER
);
";

/// A job as the journal keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub state: String,
    pub folder: String,
    pub opened_utc: Option<String>,
    pub closed_utc: Option<String>,
}

/// One test's outcome in one run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRow {
    pub seq: i64,
    pub package: String,
    pub target: String,
    pub test: String,
    pub outcome: String,
    pub millis: Option<i64>,
}

/// What a rebuild put back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rebuilt {
    /// Receipt files the journal had no row for.
    pub receipts: usize,
    /// The highest receipt number after it: where the sequence continues.
    pub last: i64,
}

/// The journal: receipts, jobs, test results. Its sequence is a checked
/// counter (uh-tom-mick), brought up to the last receipt by the one writer.
pub struct Journal {
    pub path: PathBuf,
    conn: Connection,
    seq: AtomicCounter,
}

fn jerr(e: impl std::fmt::Display) -> crate::Error {
    Refusal::Journal.refuse(e)
}

fn receipt_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<Receipt> {
    let outputs: String = row.get("outputs")?;
    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_string(),
        seq: row.get("seq")?,
        utc: row.get("utc")?,
        command: row.get("command")?,
        name: row.get("name")?,
        spec: row.get("spec")?,
        spec_hash: row.get("spec_hash")?,
        code_digest: row.get("code_digest")?,
        index_key: row.get("index_key")?,
        plan_id: row.get("plan_id")?,
        plan_digest: row.get("plan_digest")?,
        operation_digest: row.get("operation_digest")?,
        receipt_digest: row.get("receipt_digest")?,
        line: row.get("line")?,
        exit_code: row.get("exit_code")?,
        accepted: row.get::<_, i64>("accepted")? != 0,
        disposition: row.get("disposition")?,
        reason: row.get("reason")?,
        outputs: serde_json::from_str(&outputs).unwrap_or_default(),
        job: row.get("job")?,
        reused_seq: row.get("reused_seq")?,
        millis: row.get("millis")?,
        runner: row.get("runner")?,
    })
}

/// One receipt as a `runs` row, under `seq`; with `or_ignore`, a row already there is kept.
fn insert_run(conn: &Connection, r: &Receipt, seq: i64, or_ignore: bool) -> Result<()> {
    let verb = if or_ignore { "INSERT OR IGNORE" } else { "INSERT" };
    conn.execute(
        &format!(
            "{verb} INTO runs (seq, utc, command, name, spec, spec_hash, code_digest, index_key, plan_id, plan_digest, operation_digest, receipt_digest, line, exit_code, accepted, disposition, reason, outputs, job, reused_seq, millis, runner)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)"
        ),
        params![
            seq,
            r.utc,
            r.command,
            r.name,
            r.spec,
            r.spec_hash,
            r.code_digest,
            r.index_key,
            r.plan_id,
            r.plan_digest,
            r.operation_digest,
            r.receipt_digest,
            r.line,
            r.exit_code,
            r.accepted as i64,
            r.disposition,
            r.reason,
            r.outputs_json(),
            r.job,
            r.reused_seq,
            r.millis,
            r.runner
        ],
    )
    .map(|_| ())
    .map_err(jerr)
}

impl Receipt {
    fn outputs_json(&self) -> String {
        serde_json::to_string(&self.outputs).unwrap_or_default()
    }
}

/// The receipt files in `dir`: `<seq>.json`, by number.
pub fn receipt_files(dir: &Path) -> Vec<(i64, PathBuf)> {
    let mut files: Vec<(i64, PathBuf)> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter_map(|p| p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<i64>().ok()).map(|n| (n, p))).collect())
        .unwrap_or_default();
    files.sort();
    files
}

impl Journal {
    /// The journal at `path`, its tables made if new; a journal from before
    /// the runner field gets its column, every older run a line run.
    pub fn open(path: &Path) -> Result<Journal> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Refusal::Write.refuse(format!("{}: {e}", dir.display())))?;
        }
        let conn = Connection::open(path).map_err(|e| jerr(format!("{}: {e}", path.display())))?;
        conn.busy_timeout(JOURNAL_BUSY).map_err(jerr)?;
        conn.execute_batch(JOURNAL_SCHEMA).map_err(jerr)?;
        let has_runner: bool = conn.prepare("SELECT 1 FROM pragma_table_info('runs') WHERE name = 'runner'").and_then(|mut s| s.exists([])).map_err(jerr)?;
        if !has_runner {
            conn.execute_batch("ALTER TABLE runs ADD COLUMN runner TEXT NOT NULL DEFAULT 'line';").map_err(jerr)?;
        }
        let last: i64 = conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM runs", [], |r| r.get(0)).map_err(jerr)?;
        Ok(Journal { path: path.to_path_buf(), conn, seq: AtomicCounter::new(last.max(0) as u64) })
    }

    /// The journal's connection, for a caller that keeps tables of its own in the same file.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// `f` inside one writing transaction (`BEGIN IMMEDIATE`): committed when it returns, rolled back when it refuses.
    pub fn writing<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        self.conn.execute_batch("BEGIN IMMEDIATE").map_err(jerr)?;
        match f(&self.conn) {
            Ok(v) => {
                self.conn.execute_batch("COMMIT").map_err(jerr)?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    /// The counter brought up to `last`, then advanced: the next number.
    fn next_above(&self, last: i64) -> Result<i64> {
        let have = self.seq.load();
        if (last.max(0) as u64) > have {
            self.seq.add(last as u64 - have).map_err(|e| jerr(format!("{e:?}")))?;
        }
        Ok(self.seq.increment().map_err(|e| jerr(format!("{e:?}")))? as i64)
    }

    fn last_row(conn: &Connection) -> Result<i64> {
        conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM runs", [], |r| r.get(0)).map_err(jerr)
    }

    /// Records a run; the receipt comes back with its sequence number.
    pub fn record(&self, mut r: Receipt) -> Result<Receipt> {
        let seq = self.writing(|conn| {
            let n = self.next_above(Self::last_row(conn)?)?;
            insert_run(conn, &r, n, false)?;
            Ok(n)
        })?;
        r.seq = seq;
        Ok(r)
    }

    /// Keeps a receipt: takes the next number inside one writing transaction
    /// (above every row and every receipt file already in the folder
    /// `file_for` names), writes the receipt file for that number whole, and
    /// inserts the row. Returns the number and the file.
    pub fn keep(&self, r: &mut Receipt, file_for: impl Fn(i64) -> Result<PathBuf>) -> Result<(i64, PathBuf)> {
        self.writing(|conn| {
            let dir = file_for(1)?.parent().map(Path::to_path_buf).unwrap_or_default();
            let on_disk = receipt_files(&dir).last().map(|(n, _)| *n).unwrap_or(0);
            let n = self.next_above(Self::last_row(conn)?.max(on_disk))?;
            r.seq = n;
            let file = file_for(n)?;
            write_whole(&file, (serde_json::to_string_pretty(r).map_err(jerr)? + "\n").as_bytes())?;
            insert_run(conn, r, n, false)?;
            Ok((n, file))
        })
    }

    /// The receipt numbered `seq`.
    pub fn receipt(&self, seq: i64) -> Result<Option<Receipt>> {
        self.conn.query_row("SELECT * FROM runs WHERE seq = ?1", [seq], receipt_of).optional().map_err(jerr)
    }

    fn rows<T, P: rusqlite::Params>(&self, sql: &str, p: P, f: fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>) -> Result<Vec<T>> {
        let mut st = self.conn.prepare(sql).map_err(jerr)?;
        let rows = st.query_map(p, f).map_err(jerr)?;
        rows.collect::<rusqlite::Result<Vec<T>>>().map_err(jerr)
    }

    /// Every run, oldest first.
    pub fn runs(&self) -> Result<Vec<Receipt>> {
        self.rows("SELECT * FROM runs ORDER BY seq", [], receipt_of)
    }

    /// The runs after `seq`, oldest first.
    pub fn since(&self, seq: i64) -> Result<Vec<Receipt>> {
        self.rows("SELECT * FROM runs WHERE seq > ?1 ORDER BY seq", [seq], receipt_of)
    }

    /// The runs of one job, in order.
    pub fn runs_of_job(&self, job: &str) -> Result<Vec<Receipt>> {
        self.rows("SELECT * FROM runs WHERE job = ?1 ORDER BY seq", [job], receipt_of)
    }

    /// The readings of one spec name, oldest first (reuses left out).
    pub fn readings_of(&self, name: &str) -> Result<Vec<Receipt>> {
        self.rows("SELECT * FROM runs WHERE command = 'reading' AND name = ?1 AND reused_seq IS NULL AND accepted = 1 ORDER BY seq", [name], receipt_of)
    }

    /// The same search answered before: the newest accepted reading of this
    /// spec hash over this code, not itself a reuse.
    pub fn same_search(&self, spec_hash: &str, code_digest: &str) -> Result<Option<Receipt>> {
        self.conn
            .query_row(
                "SELECT * FROM runs WHERE command = 'reading' AND spec_hash = ?1 AND code_digest = ?2 AND accepted = 1 AND reused_seq IS NULL ORDER BY seq DESC LIMIT 1",
                params![spec_hash, code_digest],
                receipt_of,
            )
            .optional()
            .map_err(jerr)
    }

    /// Every receipt with an output under `old` (a folder, `/`-ended, relative
    /// to the root) rewritten to sit under `new`; returns them as rewritten.
    pub fn rehome(&self, old: &str, new: &str) -> Result<Vec<Receipt>> {
        let mut out = Vec::new();
        for mut r in self.runs()? {
            if !r.outputs.iter().any(|o| o.starts_with(old)) {
                continue;
            }
            r.outputs = r.outputs.iter().map(|o| o.strip_prefix(old).map(|rest| format!("{new}{rest}")).unwrap_or_else(|| o.clone())).collect();
            self.conn.execute("UPDATE runs SET outputs = ?1 WHERE seq = ?2", params![r.outputs_json(), r.seq]).map_err(jerr)?;
            out.push(r);
        }
        Ok(out)
    }

    /// How many receipts the journal holds.
    pub fn count(&self) -> Result<i64> {
        self.conn.query_row("SELECT COUNT(*) FROM runs", [], |r| r.get(0)).map_err(jerr)
    }

    pub fn put_job(&self, j: &JobRow) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO jobs (id, name, kind, state, folder, opened_utc, closed_utc) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET name = ?2, kind = ?3, state = ?4, folder = ?5, opened_utc = ?6, closed_utc = ?7",
                params![j.id, j.name, j.kind, j.state, j.folder, j.opened_utc, j.closed_utc],
            )
            .map(|_| ())
            .map_err(jerr)
    }

    /// A job by its id or its name (the newest of that name).
    pub fn job(&self, id_or_name: &str) -> Result<Option<JobRow>> {
        self.conn
            .query_row("SELECT id, name, kind, state, folder, opened_utc, closed_utc FROM jobs WHERE id = ?1 OR name = ?1 ORDER BY opened_utc DESC LIMIT 1", params![id_or_name], |r| {
                Ok(JobRow { id: r.get(0)?, name: r.get(1)?, kind: r.get(2)?, state: r.get(3)?, folder: r.get(4)?, opened_utc: r.get(5)?, closed_utc: r.get(6)? })
            })
            .optional()
            .map_err(jerr)
    }

    pub fn put_test(&self, t: &TestRow) -> Result<()> {
        self.conn
            .execute("INSERT INTO tests (seq, package, target, test, outcome, millis) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![t.seq, t.package, t.target, t.test, t.outcome, t.millis])
            .map(|_| ())
            .map_err(jerr)
    }

    pub fn tests(&self) -> Result<Vec<TestRow>> {
        self.rows("SELECT seq, package, target, test, outcome, millis FROM tests ORDER BY seq", [], |r| Ok(TestRow { seq: r.get(0)?, package: r.get(1)?, target: r.get(2)?, test: r.get(3)?, outcome: r.get(4)?, millis: r.get(5)? }))
    }

    pub fn tests_of_runs(&self, seqs: &[i64]) -> Result<Vec<TestRow>> {
        Ok(self.tests()?.into_iter().filter(|t| seqs.contains(&t.seq)).collect())
    }

    /// The journal brought up to the receipt files a checkout carries: every
    /// `receipts_dir/<seq>.json` the journal has no row for is put back under
    /// its own number, inside one writing transaction, and `also` is called
    /// for each with the transaction's connection (a caller's own index of the
    /// run goes in with it). The sequence continues from the highest receipt.
    /// The journal is machine state, never committed; the receipt files are
    /// the record. A journal that holds every receipt is left as it is.
    pub fn rebuild_from(&self, receipts_dir: &Path, mut also: impl FnMut(&Connection, &Receipt) -> Result<()>) -> Result<Rebuilt> {
        let have: std::collections::BTreeSet<i64> = self.rows("SELECT seq FROM runs", [], |r| r.get::<_, i64>(0))?.into_iter().collect();
        let files = receipt_files(receipts_dir);
        let mut out = Rebuilt::default();
        self.writing(|conn| {
            for (n, file) in files.iter().filter(|(n, _)| !have.contains(n)) {
                let Ok(text) = std::fs::read_to_string(file) else { continue };
                let Ok(mut r) = serde_json::from_str::<Receipt>(&text) else { continue };
                r.seq = *n;
                insert_run(conn, &r, *n, true)?;
                out.receipts += 1;
                also(conn, &r)?;
            }
            Ok(())
        })?;
        out.last = Self::last_row(&self.conn)?;
        let at = self.seq.load();
        if (out.last as u64) > at {
            self.seq.add(out.last as u64 - at).map_err(|e| jerr(format!("{e:?}")))?;
        }
        Ok(out)
    }
}
