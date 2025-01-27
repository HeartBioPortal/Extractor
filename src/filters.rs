//! Filter implementations for CSV data processing.
//! Provides a flexible filtering system for biological data analysis.

use std::str::FromStr;
use regex::Regex;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::{ExtractorError, FilterErrorKind};
use crate::Result;

/// Trait for implementing filters
pub trait Filter: Send + Sync {
    /// Apply the filter to a row of data
    fn apply(&self, row: &[u8], headers: &HashMap<String, usize>) -> Result<bool>;
    
    /// Get the name of the column this filter operates on
    fn column_name(&self) -> &str;
    
    /// Get a description of the filter
    fn description(&self) -> String;
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
    /// Get a description of the filter condition
    pub fn description(&self, column: &str) -> String {
        match self {
            FilterCondition::Equals(target) => format!("{} equals '{}'", column, target),
            FilterCondition::Contains(substring) => format!("{} contains '{}'", column, substring),
            FilterCondition::Regex(pattern) => format!("{} matches regex '{}'", column, pattern),
            FilterCondition::Numeric(num_condition) => match num_condition {
                NumericCondition::GreaterThan(v) => format!("{} > {}", column, v),
                NumericCondition::LessThan(v) => format!("{} < {}", column, v),
                NumericCondition::Equal(v) => format!("{} = {}", column, v),
                NumericCondition::NotEqual(v) => format!("{} != {}", column, v),
            },
            FilterCondition::OneOf(values) => format!("{} in {:?}", column, values),
            FilterCondition::Range(range) => format!(
                "{} {} {} and {} {}",
                column,
                if range.inclusive { ">=" } else { ">" },
                range.min,
                if range.inclusive { "<=" } else { "<" },
                range.max
            ),
            FilterCondition::Empty => format!("{} is empty", column),
            FilterCondition::NotEmpty => format!("{} is not empty", column),
        }
    }
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
    pub inclusive: bool,
}

/// Basic column filter implementation
#[derive(Debug)]
pub struct ColumnFilter {
    column: String,
    condition: FilterCondition,
    cached_regex: Option<Regex>,
}

impl ColumnFilter {
    /// Create a new column filter
    pub fn new(column: String, condition: FilterCondition) -> Result<Self> {
        let mut filter = Self {
            column,
            condition: condition.clone(),
            cached_regex: None,
        };
        
        // Pre-compile regex if needed
        if let FilterCondition::Regex(pattern) = condition {
            filter.cached_regex = Some(Regex::new(&pattern).map_err(|_| {
                ExtractorError::filter_error(
                    FilterErrorKind::InvalidRegex,
                    Some(filter.column.clone())
                )
            })?);
        }
        
        Ok(filter)
    }

    /// Helper method to get column value
    fn get_column_value<'a>(&self, row: &'a [u8], headers: &HashMap<String, usize>) -> Result<&'a [u8]> {
        let col_idx = headers.get(&self.column).ok_or_else(|| {
            ExtractorError::ColumnNotFound(self.column.clone())
        })?;
        
        Ok(&row[*col_idx])
    }

    /// Parse numeric value from bytes
    fn parse_numeric(&self, value: &[u8]) -> Result<f64> {
        std::str::from_utf8(value)
            .map_err(|_| ExtractorError::InvalidDataFormat {
                column: self.column.clone(),
                message: "Invalid UTF-8".to_string(),
                row: None,
            })?
            .trim()
            .parse::<f64>()
            .map_err(|_| ExtractorError::InvalidDataFormat {
                column: self.column.clone(),
                message: "Invalid numeric value".to_string(),
                row: None,
            })
    }
}

impl Filter for ColumnFilter {
    fn apply(&self, row: &[u8], headers: &HashMap<String, usize>) -> Result<bool> {
        let value = self.get_column_value(row, headers)?;
        
        match &self.condition {
            FilterCondition::Equals(target) => {
                Ok(value == target.as_bytes())
            },
            FilterCondition::Contains(substring) => {
                Ok(value.windows(substring.len()).any(|window| window == substring.as_bytes()))
            },
            FilterCondition::Regex(_) => {
                let str_value = std::str::from_utf8(value).map_err(|_| {
                    ExtractorError::InvalidDataFormat {
                        column: self.column.clone(),
                        message: "Invalid UTF-8".to_string(),
                        row: None,
                    }
                })?;
                Ok(self.cached_regex.as_ref().unwrap().is_match(str_value))
            },
            FilterCondition::Numeric(num_condition) => {
                let numeric_value = self.parse_numeric(value)?;
                match num_condition {
                    NumericCondition::GreaterThan(threshold) => Ok(numeric_value > *threshold),
                    NumericCondition::LessThan(threshold) => Ok(numeric_value < *threshold),
                    NumericCondition::Equal(threshold) => Ok((numeric_value - threshold).abs() < f64::EPSILON),
                    NumericCondition::NotEqual(threshold) => Ok((numeric_value - threshold).abs() >= f64::EPSILON),
                }
            },
            FilterCondition::OneOf(values) => {
                Ok(values.iter().any(|v| value == v.as_bytes()))
            },
            FilterCondition::Range(range) => {
                let numeric_value = self.parse_numeric(value)?;
                if range.inclusive {
                    Ok(numeric_value >= range.min && numeric_value <= range.max)
                } else {
                    Ok(numeric_value > range.min && numeric_value < range.max)
                }
            },
            FilterCondition::Empty => {
                Ok(value.is_empty() || value.iter().all(|&b| b.is_ascii_whitespace()))
            },
            FilterCondition::NotEmpty => {
                Ok(!value.is_empty() && !value.iter().all(|&b| b.is_ascii_whitespace()))
            },
        }
    }

    fn column_name(&self) -> &str {
        &self.column
    }

    fn description(&self) -> String {
        match &self.condition {
            FilterCondition::Equals(target) => format!("{} equals '{}'", self.column, target),
            FilterCondition::Contains(substring) => format!("{} contains '{}'", self.column, substring),
            FilterCondition::Regex(pattern) => format!("{} matches regex '{}'", self.column, pattern),
            FilterCondition::Numeric(num_condition) => {
                match num_condition {
                    NumericCondition::GreaterThan(v) => format!("{} > {}", self.column, v),
                    NumericCondition::LessThan(v) => format!("{} < {}", self.column, v),
                    NumericCondition::Equal(v) => format!("{} = {}", self.column, v),
                    NumericCondition::NotEqual(v) => format!("{} != {}", self.column, v),
                }
            },
            FilterCondition::OneOf(values) => format!("{} in {:?}", self.column, values),
            FilterCondition::Range(range) => {
                format!("{} {} {} and {} {}", 
                    self.column,
                    if range.inclusive { ">=" } else { ">" },
                    range.min,
                    if range.inclusive { "<=" } else { "<" },
                    range.max
                )
            },
            FilterCondition::Empty => format!("{} is empty", self.column),
            FilterCondition::NotEmpty => format!("{} is not empty", self.column),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_headers() -> HashMap<String, usize> {
        let mut headers = HashMap::new();
        headers.insert("name".to_string(), 0);
        headers.insert("value".to_string(), 1);
        headers
    }

    #[test]
    fn test_equals_filter() -> Result<()> {
        let filter = ColumnFilter::new(
            "name".to_string(),
            FilterCondition::Equals("test".to_string())
        )?;
        let headers = setup_test_headers();
        
        assert!(filter.apply("test,123".as_bytes(), &headers)?);
        assert!(!filter.apply("other,123".as_bytes(), &headers)?);
        Ok(())
    }

    #[test]
    fn test_numeric_filter() -> Result<()> {
        let filter = ColumnFilter::new(
            "value".to_string(),
            FilterCondition::Numeric(NumericCondition::GreaterThan(100.0))
        )?;
        let headers = setup_test_headers();
        
        assert!(filter.apply("test,150".as_bytes(), &headers)?);
        assert!(!filter.apply("test,50".as_bytes(), &headers)?);
        Ok(())
    }
}