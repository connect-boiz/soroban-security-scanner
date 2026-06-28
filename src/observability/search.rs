//! Log parsing and indexing for efficient search.
//!
//! A lightweight inverted index over level, trace id and structured fields,
//! enabling fast lookups ("all logs for this trace", "all errors", "all logs
//! where status=500") without scanning every record — the local analogue of an
//! ELK index.

use crate::observability::level::LogLevel;
use crate::observability::record::LogRecord;
use std::collections::HashMap;

/// A query against the log index.
#[derive(Debug, Clone, Default)]
pub struct LogQuery {
    /// Filter by exact level.
    pub level: Option<LogLevel>,
    /// Filter by trace id.
    pub trace_id: Option<String>,
    /// Filter by an exact field key/value.
    pub field: Option<(String, String)>,
}

impl LogQuery {
    /// An empty query (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a level filter.
    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Sets a trace-id filter.
    pub fn trace(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Sets a field filter.
    pub fn field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.field = Some((key.into(), value.into()));
        self
    }
}

/// An inverted index over a set of stored records.
#[derive(Default)]
pub struct LogIndex {
    records: Vec<LogRecord>,
    by_level: HashMap<LogLevel, Vec<usize>>,
    by_trace: HashMap<String, Vec<usize>>,
    by_field: HashMap<(String, String), Vec<usize>>,
}

impl LogIndex {
    /// Creates an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of indexed records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Indexes a record.
    pub fn index(&mut self, record: LogRecord) {
        let id = self.records.len();
        self.by_level.entry(record.level).or_default().push(id);
        if let Some(trace) = &record.trace_id {
            self.by_trace.entry(trace.clone()).or_default().push(id);
        }
        for (k, v) in &record.fields {
            self.by_field
                .entry((k.clone(), v.clone()))
                .or_default()
                .push(id);
        }
        self.records.push(record);
    }

    /// Runs a query, returning matching records. Uses the narrowest available
    /// index as the candidate set, then filters by the remaining predicates.
    pub fn search(&self, query: &LogQuery) -> Vec<&LogRecord> {
        // Pick the most selective index posting list as candidates.
        let candidates: Option<Vec<usize>> = [
            query
                .trace_id
                .as_ref()
                .and_then(|t| self.by_trace.get(t))
                .cloned(),
            query
                .field
                .as_ref()
                .and_then(|f| self.by_field.get(f))
                .cloned(),
            query.level.and_then(|l| self.by_level.get(&l)).cloned(),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|v| v.len());

        let ids: Vec<usize> = match candidates {
            Some(ids) => ids,
            None => (0..self.records.len()).collect(),
        };

        ids.into_iter()
            .map(|i| &self.records[i])
            .filter(|r| self.matches(r, query))
            .collect()
    }

    fn matches(&self, r: &LogRecord, q: &LogQuery) -> bool {
        if let Some(level) = q.level {
            if r.level != level {
                return false;
            }
        }
        if let Some(trace) = &q.trace_id {
            if r.trace_id.as_deref() != Some(trace.as_str()) {
                return false;
            }
        }
        if let Some((k, v)) = &q.field {
            if r.fields.get(k).map(|fv| fv.as_str()) != Some(v.as_str()) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::context::CorrelationContext;

    fn build() -> (LogIndex, String) {
        let mut idx = LogIndex::new();
        let ctx = CorrelationContext::new_root();
        idx.index(
            LogRecord::new(1, LogLevel::Info, "api", "ok")
                .with_context(&ctx)
                .with_field("status", "200"),
        );
        idx.index(
            LogRecord::new(2, LogLevel::Error, "api", "fail")
                .with_context(&ctx)
                .with_field("status", "500"),
        );
        idx.index(LogRecord::new(3, LogLevel::Info, "db", "query").with_field("status", "200"));
        (idx, ctx.trace_id)
    }

    #[test]
    fn search_by_level() {
        let (idx, _) = build();
        let errors = idx.search(&LogQuery::new().level(LogLevel::Error));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "fail");
    }

    #[test]
    fn search_by_trace_gives_full_request() {
        let (idx, trace) = build();
        let trace_logs = idx.search(&LogQuery::new().trace(trace));
        assert_eq!(trace_logs.len(), 2); // both records sharing the trace
    }

    #[test]
    fn search_by_field() {
        let (idx, _) = build();
        let ok = idx.search(&LogQuery::new().field("status", "200"));
        assert_eq!(ok.len(), 2);
    }

    #[test]
    fn combined_filters_narrow() {
        let (idx, trace) = build();
        let q = LogQuery::new()
            .trace(trace)
            .level(LogLevel::Info)
            .field("status", "200");
        let res = idx.search(&q);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].target, "api");
    }

    #[test]
    fn empty_query_matches_all() {
        let (idx, _) = build();
        assert_eq!(idx.search(&LogQuery::new()).len(), 3);
    }
}
