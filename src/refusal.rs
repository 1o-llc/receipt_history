// Copyright © 2026 1o. All rights reserved.

//! Every refusal `receipt_history` gives, declared once as liaise codes: the
//! code is what a caller matches on, the sentence what a person reads, the
//! detail what was refused.

use liaise::{Liaise, LiaiseCodes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, LiaiseCodes)]
#[liaise(prefix = "RCPT", domain = "receipt_history")]
pub enum Refusal {
    #[liaise(code = 1, msg = "the journal cannot be read or written")]
    Journal,
    #[liaise(code = 2, msg = "a file the journal keeps cannot be written")]
    Write,
}

impl Refusal {
    /// The error this refusal raises, with what was refused.
    pub fn refuse(self, detail: impl std::fmt::Display) -> Error {
        Error { refusal: self, detail: detail.to_string() }
    }
}

/// A refusal and what was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub refusal: Refusal,
    pub detail: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.refusal.code(), self.refusal.message(), self.detail)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
