//! Indexing strategy and advisor.
//!
//! Tracks the indexes defined on each table and recommends new ones: the
//! columns a query filters or joins on that are not already covered by a
//! leading index column. This is the static half of "proper indexing strategy".

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An index definition: leading columns on a table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef {
    /// Table the index is on.
    pub table: String,
    /// Indexed columns, in order (the first is the leading column).
    pub columns: Vec<String>,
}

impl IndexDef {
    /// Builds an index definition.
    pub fn new(table: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            table: table.into(),
            columns,
        }
    }

    /// Whether this index can serve a lookup whose leading column is `column`.
    pub fn covers_leading(&self, column: &str) -> bool {
        self.columns.first().map(|c| c == column).unwrap_or(false)
    }
}

/// A query's access shape for index analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryShape {
    /// Primary table.
    pub table: String,
    /// Columns used in equality/range filters (`WHERE`).
    pub filter_columns: Vec<String>,
    /// Columns used to join.
    pub join_columns: Vec<String>,
}

/// A recommended index to create.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexRecommendation {
    /// Table.
    pub table: String,
    /// Column that should be indexed.
    pub column: String,
    /// Why (filter or join).
    pub reason: String,
}

/// Registry of defined indexes with advice generation.
#[derive(Debug, Clone, Default)]
pub struct IndexAdvisor {
    indexes: HashMap<String, Vec<IndexDef>>,
}

impl IndexAdvisor {
    /// Creates an empty advisor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an existing index.
    pub fn add_index(&mut self, index: IndexDef) {
        self.indexes
            .entry(index.table.clone())
            .or_default()
            .push(index);
    }

    /// Whether a table has an index whose leading column is `column`.
    pub fn has_leading_index(&self, table: &str, column: &str) -> bool {
        self.indexes
            .get(table)
            .map(|idxs| idxs.iter().any(|i| i.covers_leading(column)))
            .unwrap_or(false)
    }

    /// Recommends indexes for a query shape: any filter/join column lacking a
    /// leading index.
    pub fn recommend(&self, shape: &QueryShape) -> Vec<IndexRecommendation> {
        let mut recs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut consider = |column: &str, reason: &str, recs: &mut Vec<IndexRecommendation>| {
            if !self.has_leading_index(&shape.table, column) && seen.insert(column.to_string()) {
                recs.push(IndexRecommendation {
                    table: shape.table.clone(),
                    column: column.to_string(),
                    reason: reason.to_string(),
                });
            }
        };
        for col in &shape.filter_columns {
            consider(
                col,
                "filtered in WHERE without a supporting index",
                &mut recs,
            );
        }
        for col in &shape.join_columns {
            consider(col, "used in JOIN without a supporting index", &mut recs);
        }
        recs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advisor() -> IndexAdvisor {
        let mut a = IndexAdvisor::new();
        a.add_index(IndexDef::new("users", vec!["id".to_string()]));
        a
    }

    #[test]
    fn recommends_missing_filter_index() {
        let a = advisor();
        let shape = QueryShape {
            table: "users".to_string(),
            filter_columns: vec!["email".to_string()],
            join_columns: vec![],
        };
        let recs = a.recommend(&shape);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].column, "email");
        assert!(recs[0].reason.contains("WHERE"));
    }

    #[test]
    fn covered_column_not_recommended() {
        let a = advisor();
        let shape = QueryShape {
            table: "users".to_string(),
            filter_columns: vec!["id".to_string()],
            join_columns: vec![],
        };
        assert!(a.recommend(&shape).is_empty());
    }

    #[test]
    fn join_columns_recommended() {
        let a = advisor();
        let shape = QueryShape {
            table: "orders".to_string(),
            filter_columns: vec![],
            join_columns: vec!["user_id".to_string()],
        };
        let recs = a.recommend(&shape);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].reason.contains("JOIN"));
    }

    #[test]
    fn duplicate_columns_deduped() {
        let a = advisor();
        let shape = QueryShape {
            table: "t".to_string(),
            filter_columns: vec!["x".to_string(), "x".to_string()],
            join_columns: vec!["x".to_string()],
        };
        assert_eq!(a.recommend(&shape).len(), 1);
    }
}
