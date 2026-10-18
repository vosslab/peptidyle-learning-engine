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
#[serde(try_from = "u16", into = "u16")]
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

impl TryFrom<u16> for PageSize {
    type Error = PaginationError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PageSize> for u16 {
    fn from(value: PageSize) -> Self {
        value.get()
    }
}

/// Validated maximum number of rows returned by one broad discovery operation.
///
/// Discovery is intentionally wider than ordinary cursor lists.  Keeping this
/// type separate prevents a discovery UI choice from widening unrelated task,
/// history, or administrative list boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct DiscoveryPageSize(u16);

impl DiscoveryPageSize {
    /// Largest Question, Question Pool, or Blueprint discovery page.
    pub const MAX: u16 = question_model::MAX_DISCOVERY_PAGE_SIZE as u16;

    /// Validates a requested discovery page size in `1..=250`.
    pub fn new(value: u16) -> Result<Self, PaginationError> {
        if (1..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(PaginationError::InvalidDiscoveryPageSize { value })
        }
    }

    /// Validated discovery row limit.
    pub fn get(&self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for DiscoveryPageSize {
    type Error = PaginationError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DiscoveryPageSize> for u16 {
    fn from(value: DiscoveryPageSize) -> Self {
        value.get()
    }
}

/// A validated one-based page number for an ordinary numbered list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct PageNumber(u32);

impl PageNumber {
    /// First page of a result set.
    pub const FIRST: Self = Self(1);

    /// Validates a one-based page number.
    pub fn new(value: u32) -> Result<Self, PaginationError> {
        if value == 0 {
            Err(PaginationError::InvalidPageNumber { value })
        } else {
            Ok(Self(value))
        }
    }

    /// One-based page number.
    pub fn get(&self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for PageNumber {
    type Error = PaginationError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<PageNumber> for u32 {
    fn from(value: PageNumber) -> Self {
        value.get()
    }
}

/// Numbered request for an ordinary list with the ordinary page-size bound.
///
/// Domain list routes use this during the cursor-to-numbered migration. Facet choice lists retain
/// their independent cursor requests because they do not expose collection pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberedPageRequest {
    /// One-based page requested by the client.
    pub page_number: PageNumber,
    /// Validated maximum row count.
    pub page_size: PageSize,
}

impl NumberedPageRequest {
    /// Starts at the first numbered page.
    pub fn first(page_size: PageSize) -> Self {
        Self {
            page_number: PageNumber::FIRST,
            page_size,
        }
    }
}

/// Numbered request for broad discovery with the wider discovery page-size bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberedDiscoveryPageRequest {
    /// One-based page requested by the client.
    pub page_number: PageNumber,
    /// Validated discovery row count.
    pub page_size: DiscoveryPageSize,
}

impl NumberedDiscoveryPageRequest {
    /// Starts at the first numbered discovery page.
    pub fn first(page_size: DiscoveryPageSize) -> Self {
        Self {
            page_number: PageNumber::FIRST,
            page_size,
        }
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

/// Cursor plus mandatory bounded discovery row limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryPageRequest {
    /// Cursor returned by the prior discovery page, or `None` for the first page.
    pub after: Option<Cursor>,
    /// Validated discovery row count.
    pub size: DiscoveryPageSize,
}

impl DiscoveryPageRequest {
    /// Starts a bounded discovery cursor sequence.
    pub fn first(size: DiscoveryPageSize) -> Self {
        Self { after: None, size }
    }

    /// Continues after a cursor returned by the prior discovery page.
    pub fn after(cursor: Cursor, size: DiscoveryPageSize) -> Self {
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

/// A resolved numbered result page after the store applies scope, filters, and ordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumberedPage<T, Size> {
    /// Rows in stable key order for the resolved page.
    pub items: Vec<T>,
    /// One-based requested page after the store clamps it to the matching result set.
    pub page_number: PageNumber,
    /// The validated request page size.
    pub page_size: Size,
    /// Number of matching rows before the result is sliced into pages.
    pub total_count: u64,
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
    /// Requested discovery row count was zero or exceeded the discovery bound.
    InvalidDiscoveryPageSize {
        /// Rejected requested size.
        value: u16,
    },
    /// Requested numbered page zero.
    InvalidPageNumber {
        /// Rejected page number.
        value: u32,
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
            Self::InvalidDiscoveryPageSize { value } => {
                write!(
                    formatter,
                    "discovery page size must be between 1 and 250, got {value}"
                )
            }
            Self::InvalidPageNumber { value } => {
                write!(formatter, "page number must be at least 1, got {value}")
            }
        }
    }
}

impl std::error::Error for PaginationError {}
