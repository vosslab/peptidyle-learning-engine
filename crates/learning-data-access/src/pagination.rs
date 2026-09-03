//! Bounded cursor pagination shared by every list operation.

use serde::{Deserialize, Serialize};

/// Opaque continuation token returned by a prior page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor(String);

impl Cursor {
    /// Parses a non-empty cursor received back from a client.
    ///
    /// # Errors
    ///
    /// Returns [`PaginationError`] for an empty token.
    pub fn parse(token: String) -> Result<Self, PaginationError> {
        if token.is_empty() {
            Err(PaginationError::EmptyCursor)
        } else {
            Ok(Self(token))
        }
    }

    /// Opaque token used by a backend to resume its query.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated maximum number of rows returned by one list operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSize(u16);

impl PageSize {
    /// Largest list page any backend may return.
    pub const MAX: u16 = 100;

    /// Validates a requested page size in `1..=100`.
    ///
    /// # Errors
    ///
    /// Returns [`PaginationError`] when the request is zero or unbounded.
    pub fn new(value: u16) -> Result<Self, PaginationError> {
        if (1..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(PaginationError::InvalidPageSize { value })
        }
    }

    /// Validated row limit.
    pub fn get(&self) -> u16 {
        self.0
    }
}

/// Cursor plus mandatory bounded row limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest {
    /// Cursor returned by the prior page, or `None` for the first page.
    pub after: Option<Cursor>,
    /// Validated maximum row count.
    pub size: PageSize,
}

impl PageRequest {
    /// Starts a bounded cursor sequence.
    pub fn first(size: PageSize) -> Self {
        Self { after: None, size }
    }

    /// Continues after a cursor returned by the prior page.
    pub fn after(cursor: Cursor, size: PageSize) -> Self {
        Self {
            after: Some(cursor),
            size,
        }
    }
}

/// One bounded result page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    /// Rows in stable key order.
    pub items: Vec<T>,
    /// Cursor for a following page, or `None` at the end.
    pub next_cursor: Option<Cursor>,
}

/// Rejected pagination input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationError {
    /// A cursor token carried no stable key.
    EmptyCursor,
    /// Requested row count was zero or exceeded the hard bound.
    InvalidPageSize {
        /// Rejected requested size.
        value: u16,
    },
}

impl std::fmt::Display for PaginationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCursor => write!(formatter, "cursor must not be empty"),
            Self::InvalidPageSize { value } => {
                write!(
                    formatter,
                    "page size must be between 1 and 100, got {value}"
                )
            }
        }
    }
}

impl std::error::Error for PaginationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbounded_page_size_is_rejected() {
        assert_eq!(
            PageSize::new(PageSize::MAX + 1),
            Err(PaginationError::InvalidPageSize {
                value: PageSize::MAX + 1,
            })
        );
    }
}
