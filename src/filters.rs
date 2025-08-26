//! Filter implementations for CSV data processing.
//! Provides a flexible filtering system for biological data analysis.
//!
//! Improvements over prior version:
//! - Uses `csv::ByteRecord` instead of raw `&[u8]` so “column” access is correct.
//! - Caches column index (lazy) and precompiles regex for speed.
//! - Robust numeric parsing (trim + friendly errors).
//! - Fast substring search via `memchr::memmem` (no manual windowing).
//! - Treats common empty/NA tokens as empty when desired.
//! - Clearer descriptions and tighter tests.

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::str;
use std::sync::OnceLock;

use csv::ByteRecord;
use serde::{Deserialize, Serialize};

use crate::error::{ExtractorError, FilterErrorKind};
use crate::Result;

/// Trait for implementing filters
pub trait Filter: Send + Sync {
    /// Apply the filter to a row of data
    fn apply(&self, row: &ByteRecord, headers: &HashMap<String, usize>) -> Result<bool>;

    /// Get the name of the column this filter operates on
    fn column_name(&self) -> &str;

    /// Get a description of the filter
    fn description(&self) -> String;
}

/// Numeric comparison conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NumericCondition {
    GreaterThan(f64),
    LessThan(f64),
    Equal(f64),
    NotEqual(f64),
}

/// Range condition for numeric values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeCondition {
    pub min: f64,
    pub max: f64,
    /// When true, min/max are inclusive (>= and <=). When false, exclusive.
    pub inclusive: bool,
}

/// Filter condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterCondition {
    Equals(String),
    Contains(String),
    Regex(String),
    Numeric(NumericCondition),
    OneOf(Vec<String>),
    Range(RangeCondition),
    Empty,
    NotEmpty,
}

impl FilterCondition {
    /// Get a human-readable description of the filter condition
    pub fn description(&self, column: &str) -> String {
        match self {
            FilterCondition::Equals(target) => format!("{column} equals '{target}'"),
            FilterCondition::Contains(substring) => format!("{column} contains '{substring}'"),
            FilterCondition::Regex(pattern) => format!("{column} matches regex '{pattern}'"),
            FilterCondition::Numeric(num_condition) => match num_condition {
                NumericCondition::GreaterThan(v) => format!("{column} > {v}"),
                NumericCondition::LessThan(v) => format!("{column} < {v}"),
                NumericCondition::Equal(v) => format!("{column} = {v}"),
                NumericCondition::NotEqual(v) => format!("{column} != {v}"),
            },
            FilterCondition::OneOf(values) => format!("{column} in {:?}", values),
            FilterCondition::Range(range) => format!(
                "{column} {} {} and {} {}",
                if range.inclusive { ">=" } else { ">" },
                range.min,
                if range.inclusive { "<=" } else { "<" },
                range.max
            ),
            FilterCondition::Empty => format!("{column} is empty"),
            FilterCondition::NotEmpty => format!("{column} is not empty"),
        }
    }
}

/// Basic column filter implementation
#[derive(Debug)]
pub struct ColumnFilter {
    column: String,
    condition: FilterCondition,

    /// Cached/derived data for fast evaluation
    col_idx: OnceLock<usize>,
    cached_regex: Option<Regex>,
    one_of_set: Option<HashSet<Vec<u8>>>,

    /// Tokens that should be treated as "empty" (case-insensitive).
    /// Defaults include "", "NA", "N/A", "NULL", ".", "NaN".
    empty_tokens: HashSet<String>,
}

impl ColumnFilter {
    /// Create a new column filter. Header indices are resolved lazily on first apply().
    pub fn new(column: String, condition: FilterCondition) -> Result<Self> {
        // Pre-compile regex if needed
        let cached_regex = if let FilterCondition::Regex(pattern) = &condition {
            Some(
                Regex::new(pattern).map_err(|_| {
                    ExtractorError::filter_error(
                        FilterErrorKind::InvalidRegex,
                        Some(column.clone()),
                    )
                })?,
            )
        } else {
            None
        };

        // Pre-build a HashSet for OneOf for faster membership tests
        let one_of_set = if let FilterCondition::OneOf(values) = &condition {
            let set: HashSet<Vec<u8>> = values.iter().map(|v| v.as_bytes().to_vec()).collect();
            Some(set)
        } else {
            None
        };

        // Default empty/NA tokens
        let empty_tokens = [
            "", "NA", "N/A", "NULL", ".", "NaN", "None", "null", "nan",
        ]
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect::<HashSet<_>>();

        Ok(Self {
            column,
            condition,
            col_idx: OnceLock::new(),
            cached_regex,
            one_of_set,
            empty_tokens,
        })
    }

    /// Optionally customize which tokens count as "empty"
    pub fn with_empty_tokens(mut self, tokens: impl IntoIterator<Item = String>) -> Self {
        self.empty_tokens = tokens
            .into_iter()
            .map(|s| s.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        self
    }

    #[inline]
    fn resolve_col_idx(&self, headers: &HashMap<String, usize>) -> Result<usize> {
        if let Some(idx) = self.col_idx.get() {
            return Ok(*idx);
        }
        let idx = *headers
            .get(&self.column)
            .ok_or_else(|| ExtractorError::ColumnNotFound(self.column.clone()))?;
        // Set once; subsequent calls are fast
        let _ = self.col_idx.set(idx);
        Ok(idx)
    }

    #[inline]
    fn get_value<'a>(&self, row: &'a ByteRecord, headers: &HashMap<String, usize>) -> Result<&'a [u8]> {
        let idx = self.resolve_col_idx(headers)?;
        row.get(idx).ok_or_else(|| {
            ExtractorError::InvalidDataFormat {
                column: self.column.clone(),
                message: format!("Row has no field at index {idx}"),
                row: None,
            }
        })
    }

    #[inline]
    fn parse_numeric(&self, value: &[u8]) -> Result<f64> {
        let s = str::from_utf8(value).map_err(|_| ExtractorError::InvalidDataFormat {
            column: self.column.clone(),
            message: "Invalid UTF-8".to_string(),
            row: None,
        })?;
        let s = s.trim();
        s.parse::<f64>().map_err(|_| ExtractorError::InvalidDataFormat {
            column: self.column.clone(),
            message: format!("Invalid numeric value: '{s}'"),
            row: None,
        })
    }

    #[inline]
    fn is_empty_token(&self, value: &[u8]) -> bool {
        // Trim ASCII whitespace, then case-insensitive token check
        let trimmed = trim_ascii(value);
        if trimmed.is_empty() {
            return true;
        }
        // Lowercase ASCII only (common CSVs); avoid alloc if possible
        let lower = ascii_to_lower_lossy(trimmed);
        self.empty_tokens.contains(&lower)
    }

    #[inline]
    fn approx_eq(a: f64, b: f64) -> bool {
        // Relative tolerance to avoid strict bitwise equality woes.
        let tol = 1e-12_f64.max(1e-12 * a.abs().max(b.abs()));
        (a - b).abs() <= tol
    }
}

impl Filter for ColumnFilter {
    fn apply(&self, row: &ByteRecord, headers: &HashMap<String, usize>) -> Result<bool> {
        use memchr::memmem;

        let value = self.get_value(row, headers)?;

        match &self.condition {
            FilterCondition::Equals(target) => Ok(value == target.as_bytes()),
            FilterCondition::Contains(substring) => {
                Ok(memmem::find(value, substring.as_bytes()).is_some())
            }
            FilterCondition::Regex(_) => {
                let s = str::from_utf8(value).map_err(|_| ExtractorError::InvalidDataFormat {
                    column: self.column.clone(),
                    message: "Invalid UTF-8".to_string(),
                    row: None,
                })?;
                Ok(self.cached_regex.as_ref().expect("regex precompiled").is_match(s))
            }
            FilterCondition::Numeric(cond) => {
                let x = self.parse_numeric(value)?;
                let pass = match cond {
                    NumericCondition::GreaterThan(t) => x > *t,
                    NumericCondition::LessThan(t) => x < *t,
                    NumericCondition::Equal(t) => Self::approx_eq(x, *t),
                    NumericCondition::NotEqual(t) => !Self::approx_eq(x, *t),
                };
                Ok(pass)
            }
            FilterCondition::OneOf(_) => {
                let set = self.one_of_set.as_ref().expect("one_of_set prebuilt");
                Ok(set.contains(value))
            }
            FilterCondition::Range(r) => {
                let x = self.parse_numeric(value)?;
                let lower_ok = if r.inclusive { x >= r.min } else { x > r.min };
                let upper_ok = if r.inclusive { x <= r.max } else { x < r.max };
                Ok(lower_ok && upper_ok)
            }
            FilterCondition::Empty => Ok(self.is_empty_token(value)),
            FilterCondition::NotEmpty => Ok(!self.is_empty_token(value)),
        }
    }

    fn column_name(&self) -> &str {
        &self.column
    }

    fn description(&self) -> String {
        self.condition.description(&self.column)
    }
}

/// Helpers

#[inline]
fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &bytes[start..end]
}

#[inline]
fn ascii_to_lower_lossy(bytes: &[u8]) -> String {
    // Avoid allocation per row by limiting to ASCII fold; OK for typical CSVs.
    let mut s = String::with_capacity(bytes.len());
    for &b in bytes {
        s.push((b as char).to_ascii_lowercase());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::ByteRecord;

    fn headers() -> HashMap<String, usize> {
        let mut h = HashMap::new();
        h.insert("name".to_string(), 0);
        h.insert("value".to_string(), 1);
        h
    }

    fn row(name: &str, value: &str) -> ByteRecord {
        ByteRecord::from(vec![name, value])
    }

    #[test]
    fn test_equals_filter() -> Result<()> {
        let f = ColumnFilter::new(
            "name".to_string(),
            FilterCondition::Equals("test".to_string()),
        )?;
        let h = headers();

        assert!(f.apply(&row("test", "123"), &h)?);
        assert!(!f.apply(&row("other", "123"), &h)?);
        Ok(())
    }

    #[test]
    fn test_contains_filter() -> Result<()> {
        let f = ColumnFilter::new(
            "name".to_string(),
            FilterCondition::Contains("est".to_string()),
        )?;
        let h = headers();

        assert!(f.apply(&row("test", "1"), &h)?);
        assert!(!f.apply(&row("toast", "1"), &h)?);
        Ok(())
    }

    #[test]
    fn test_regex_filter() -> Result<()> {
        let f = ColumnFilter::new(
            "name".to_string(),
            FilterCondition::Regex("^te.*$".to_string()),
        )?;
        let h = headers();

        assert!(f.apply(&row("test", "123"), &h)?);
        assert!(!f.apply(&row("best", "123"), &h)?);
        Ok(())
    }

    #[test]
    fn test_numeric_filter() -> Result<()> {
        let f = ColumnFilter::new(
            "value".to_string(),
            FilterCondition::Numeric(NumericCondition::GreaterThan(100.0)),
        )?;
        let h = headers();

        assert!(f.apply(&row("x", "150"), &h)?);
        assert!(!f.apply(&row("x", "50"), &h)?);
        Ok(())
    }

    #[test]
    fn test_range_filter_inclusive() -> Result<()> {
        let f = ColumnFilter::new(
            "value".to_string(),
            FilterCondition::Range(RangeCondition {
                min: 1.0,
                max: 2.0,
                inclusive: true,
            }),
        )?;
        let h = headers();

        assert!(f.apply(&row("x", "1.0"), &h)?);
        assert!(f.apply(&row("x", "1.5"), &h)?);
        assert!(f.apply(&row("x", "2.0"), &h)?);
        assert!(!f.apply(&row("x", "0.9999"), &h)?);
        assert!(!f.apply(&row("x", "2.0001"), &h)?);
        Ok(())
    }

    #[test]
    fn test_one_of_filter() -> Result<()> {
        let f = ColumnFilter::new(
            "name".to_string(),
            FilterCondition::OneOf(vec!["A".into(), "B".into(), "C".into()]),
        )?;
        let h = headers();

        assert!(f.apply(&row("B", "10"), &h)?);
        assert!(!f.apply(&row("Z", "10"), &h)?);
        Ok(())
    }

    #[test]
    fn test_empty_not_empty_filters() -> Result<()> {
        let empty_f = ColumnFilter::new("name".to_string(), FilterCondition::Empty)?;
        let not_empty_f = ColumnFilter::new("name".to_string(), FilterCondition::NotEmpty)?;
        let h = headers();

        assert!(empty_f.apply(&row("  ", "1"), &h)?);
        assert!(empty_f.apply(&row("NA", "1"), &h)?);
        assert!(!empty_f.apply(&row("test", "1"), &h)?);

        assert!(!not_empty_f.apply(&row("  ", "1"), &h)?);
        assert!(!not_empty_f.apply(&row("NA", "1"), &h)?);
        assert!(not_empty_f.apply(&row("value", "1"), &h)?);
        Ok(())
    }

    #[test]
    fn test_descriptions() -> Result<()> {
        let f = ColumnFilter::new(
            "value".to_string(),
            FilterCondition::Numeric(NumericCondition::LessThan(3.14)),
        )?;
        assert_eq!(f.description(), "value < 3.14");
        Ok(())
    }
}
