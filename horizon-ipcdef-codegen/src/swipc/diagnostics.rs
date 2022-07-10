use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
use codespan_reporting::files::SimpleFiles;
use itertools::Either;
use lalrpop_util::lexer::Token;
use lalrpop_util::ParseError;
use std::ops::Range;

pub fn diagnostics_and_files_from_parse_error<'source>(
    source: &'source str,
    error: ParseError<usize, Token<'source>, Vec<Diagnostic<usize>>>,
) -> (SimpleFiles<&'source str, &'source str>, Error) {
    let mut files = SimpleFiles::new();
    let file_id = files.add("/dev/stdin", source);

    let error = diagnostics_from_parse_error(file_id, source, error);

    (files, error)
}

pub fn diagnostics_from_parse_error<'source>(
    file_id: usize,
    _source: &'source str,
    error: ParseError<usize, Token<'source>, Vec<Diagnostic<usize>>>,
) -> Error {
    let diagnostic = Diagnostic::error();

    let diagnostic = match error {
        ParseError::InvalidToken { location } => diagnostic
            .with_message("Invalid token")
            .with_labels(vec![Label::primary(file_id, location..location)]),
        ParseError::UnrecognizedEOF { location, expected } => diagnostic
            .with_message("Unrecognized EOF")
            .with_labels(vec![Label::primary(file_id, location..location)])
            .with_notes(vec![format!(
                "Expected one of the following: {}",
                expected.join(", ")
            )]),
        ParseError::UnrecognizedToken {
            token: (start, t, end),
            expected,
        } => diagnostic
            .with_message(format!("Unrecognized token: {}", t))
            .with_labels(vec![Label::primary(file_id, start..end)])
            .with_notes(vec![format!(
                "Expected one of the following: {}",
                expected.join(", ")
            )]),
        ParseError::ExtraToken {
            token: (start, t, end),
        } => diagnostic
            .with_message(format!("Extra token: {}", t))
            .with_labels(vec![Label::primary(file_id, start..end)]),
        ParseError::User { error } => return error,
    };

    vec![diagnostic]
}

pub type Error = Vec<Diagnostic<usize>>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn is_diags_fatal(diags: &Error) -> bool {
    diags.iter().any(|diag| diag.severity >= Severity::Error)
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Span {
    pub file_id: usize,
    pub left: usize,
    pub right: usize,
}

impl Span {
    pub fn new(file_id: usize, left: usize, right: usize) -> Self {
        Self {
            file_id,
            left,
            right,
        }
    }

    pub fn range(&self) -> Range<usize> {
        self.left..self.right
    }

    pub fn primary_label(&self) -> Label<usize> {
        Label::primary(self.file_id, self.range())
    }
    pub fn secondary_label(&self) -> Label<usize> {
        Label::secondary(self.file_id, self.range())
    }
}

impl From<&Span> for Span {
    fn from(span: &Span) -> Self {
        *span
    }
}

pub trait DiagnosticExt<FileId> {
    fn with_primary_label(self, location: impl Into<Span>) -> Diagnostic<FileId>;
    fn with_secondary_label(
        self,
        location: impl Into<Span>,
        message: impl Into<String>,
    ) -> Diagnostic<FileId>;
}

impl DiagnosticExt<usize> for Diagnostic<usize> {
    fn with_primary_label(self, location: impl Into<Span>) -> Diagnostic<usize> {
        let location = location.into();
        self.with_labels(vec![location.primary_label()])
    }

    fn with_secondary_label(
        self,
        location: impl Into<Span>,
        message: impl Into<String>,
    ) -> Diagnostic<usize> {
        let location = location.into();
        self.with_labels(vec![location.secondary_label().with_message(message)])
    }
}

struct LazyString<F, M>
where
    F: FnOnce() -> M,
    M: Into<String>,
{
    f: Either<F, String>,
}

impl<F, M> LazyString<F, M>
where
    F: FnOnce() -> M,
    M: Into<String>,
{
    pub fn new(f: F) -> Self {
        Self { f: Either::Left(f) }
    }

    pub fn get(&mut self) -> String {
        let s = std::mem::replace(
            self,
            Self {
                f: Either::Right(String::new()),
            },
        )
        .into();

        *self = Self {
            f: Either::Right(s.clone()),
        };

        s
    }

    pub fn into(self) -> String {
        match self.f {
            Either::Left(f) => f().into(),
            Either::Right(s) => s,
        }
    }
}

pub trait DiagnosticErrorExt {
    fn context<L, M>(self, location: L, message: M) -> Error
    where
        L: Into<Span>,
        M: Into<String>;

    fn with_context<L, F, M>(self, location: L, message: F) -> Error
    where
        L: Into<Span>,
        F: FnOnce() -> M,
        M: Into<String>;
}

impl DiagnosticErrorExt for Error {
    fn context<L, M>(self, location: L, message: M) -> Error
    where
        L: Into<Span>,
        M: Into<String>,
    {
        let location = location.into();
        let message = message.into();
        self.into_iter()
            .map(|diag| diag.with_secondary_label(location, message.clone()))
            .collect()
    }

    fn with_context<L, F, M>(self, location: L, message: F) -> Error
    where
        L: Into<Span>,
        F: FnOnce() -> M,
        M: Into<String>,
    {
        let location = location.into();
        let mut message = LazyString::new(message);
        self.into_iter()
            .map(|diag| diag.with_secondary_label(location, message.get()))
            .collect()
    }
}

pub trait DiagnosticResultExt<T> {
    fn context<L, M>(self, location: L, message: M) -> Result<T>
    where
        L: Into<Span>,
        M: Into<String>;

    fn with_context<L, F, M>(self, location: L, message: F) -> Result<T>
    where
        L: Into<Span>,
        F: FnOnce() -> M,
        M: Into<String>;

    fn push(&mut self, diagnostic: Diagnostic<usize>);
    fn extend<It>(&mut self, error: It)
    where
        It: IntoIterator<Item = Diagnostic<usize>>;

    fn extend_result<K>(&mut self, result: Result<K>) -> Option<K>;
}

impl<T> DiagnosticResultExt<T> for Result<T> {
    fn context<L, M>(self, location: L, message: M) -> Result<T>
    where
        L: Into<Span>,
        M: Into<String>,
    {
        self.map_err(|e| e.context(location, message))
    }

    fn with_context<L, F, M>(self, location: L, message: F) -> Result<T>
    where
        L: Into<Span>,
        F: FnOnce() -> M,
        M: Into<String>,
    {
        self.map_err(|e| e.with_context(location, message))
    }

    fn push(&mut self, diagnostic: Diagnostic<usize>) {
        match self {
            Ok(_) => *self = Err(vec![diagnostic]),
            Err(v) => v.push(diagnostic),
        }
    }

    fn extend<It>(&mut self, error: It)
    where
        It: IntoIterator<Item = Diagnostic<usize>>,
    {
        match self {
            Ok(_) => *self = Err(error.into_iter().collect()),
            Err(v) => v.extend(error),
        }
    }

    fn extend_result<K>(&mut self, result: Result<K>) -> Option<K> {
        match result {
            Ok(r) => Some(r),
            Err(e) => {
                self.extend(e);
                None
            }
        }
    }
}
