//! SIEM integration.
//!
//! Serializes security events into the formats expected by common SIEMs and
//! ships them through a pluggable [`SiemSink`]. Supports Splunk HEC-style JSON,
//! ELK/ECS-style JSON, and ArcSight/AWS-friendly CEF. A buffering in-memory
//! sink is provided for tests and as a local fallback.

use crate::security_monitoring::event::SecurityEvent;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Target SIEM / wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiemFormat {
    /// Splunk HTTP Event Collector JSON envelope.
    SplunkHec,
    /// Elastic Common Schema JSON.
    ElkEcs,
    /// Common Event Format (ArcSight / AWS Security Hub ingest).
    Cef,
}

/// Formats a security event for the given SIEM target.
pub fn format_event(event: &SecurityEvent, format: SiemFormat) -> String {
    match format {
        SiemFormat::SplunkHec => format!(
            r#"{{"time":{},"event":{{"kind":"{:?}","component":"{:?}","severity":"{}","src_ip":"{}","user":"{}","detail":{}}}}}"#,
            event.at,
            event.kind,
            event.component,
            event.severity.as_str(),
            event.source_ip.as_deref().unwrap_or(""),
            event.principal.as_deref().unwrap_or(""),
            json_string(&event.detail),
        ),
        SiemFormat::ElkEcs => format!(
            r#"{{"@timestamp":{},"event":{{"kind":"{:?}","module":"{:?}"}},"log":{{"level":"{}"}},"source":{{"ip":"{}"}},"user":{{"name":"{}"}},"message":{}}}"#,
            event.at,
            event.kind,
            event.component,
            event.severity.as_str(),
            event.source_ip.as_deref().unwrap_or(""),
            event.principal.as_deref().unwrap_or(""),
            json_string(&event.detail),
        ),
        SiemFormat::Cef => format!(
            "CEF:0|SorobanScanner|SecurityMonitor|1.0|{:?}|{:?}|{}|src={} suser={} msg={}",
            event.kind,
            event.kind,
            event.severity.weight(),
            event.source_ip.as_deref().unwrap_or(""),
            event.principal.as_deref().unwrap_or(""),
            event.detail.replace('|', "\\|"),
        ),
    }
}

/// Minimal JSON string escaping for the detail field.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A SIEM delivery sink (HTTP forwarder in production).
pub trait SiemSink: Send + Sync {
    /// Ships a single pre-formatted record. Returns `Ok(())` on success.
    fn ship(&self, record: &str) -> Result<(), String>;
}

/// A buffering in-memory sink for tests / local fallback.
#[derive(Default)]
pub struct InMemorySiemSink {
    records: Mutex<Vec<String>>,
}

impl InMemorySiemSink {
    /// Creates an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// All shipped records.
    pub fn records(&self) -> Vec<String> {
        self.records.lock().expect("siem sink poisoned").clone()
    }

    /// Number of shipped records.
    pub fn len(&self) -> usize {
        self.records.lock().expect("siem sink poisoned").len()
    }

    /// Whether nothing has been shipped.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl SiemSink for InMemorySiemSink {
    fn ship(&self, record: &str) -> Result<(), String> {
        self.records
            .lock()
            .expect("siem sink poisoned")
            .push(record.to_string());
        Ok(())
    }
}

/// Forwards events to a SIEM in a configured format.
pub struct SiemForwarder {
    format: SiemFormat,
    sink: Box<dyn SiemSink>,
}

impl SiemForwarder {
    /// Creates a forwarder.
    pub fn new(format: SiemFormat, sink: Box<dyn SiemSink>) -> Self {
        Self { format, sink }
    }

    /// Formats and ships one event.
    pub fn forward(&self, event: &SecurityEvent) -> Result<(), String> {
        self.sink.ship(&format_event(event, self.format))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_monitoring::event::{Component, EventKind, SecuritySeverity};

    fn event() -> SecurityEvent {
        SecurityEvent::new(
            1700,
            EventKind::AttackSignature,
            Component::Network,
            SecuritySeverity::Critical,
        )
        .with_ip("8.8.8.8")
        .with_principal("mallory")
        .with_detail("payload with | pipe")
    }

    #[test]
    fn splunk_format_includes_fields() {
        let s = format_event(&event(), SiemFormat::SplunkHec);
        assert!(s.contains("\"time\":1700"));
        assert!(s.contains("CRITICAL"));
        assert!(s.contains("8.8.8.8"));
    }

    #[test]
    fn ecs_format_has_timestamp_and_level() {
        let s = format_event(&event(), SiemFormat::ElkEcs);
        assert!(s.contains("@timestamp"));
        assert!(s.contains("\"level\":\"CRITICAL\""));
    }

    #[test]
    fn cef_escapes_pipes() {
        let s = format_event(&event(), SiemFormat::Cef);
        assert!(s.starts_with("CEF:0|SorobanScanner"));
        assert!(s.contains("payload with \\| pipe"));
    }

    #[test]
    fn in_memory_sink_buffers_records() {
        let sink = InMemorySiemSink::new();
        assert!(sink.is_empty());
        sink.ship(&format_event(&event(), SiemFormat::SplunkHec))
            .unwrap();
        assert_eq!(sink.len(), 1);
    }

    #[test]
    fn forwarder_end_to_end() {
        use std::sync::Arc;
        struct Shared(Arc<InMemorySiemSink>);
        impl SiemSink for Shared {
            fn ship(&self, record: &str) -> Result<(), String> {
                self.0.ship(record)
            }
        }
        let inner = Arc::new(InMemorySiemSink::new());
        let fwd = SiemForwarder::new(SiemFormat::Cef, Box::new(Shared(Arc::clone(&inner))));
        fwd.forward(&event()).unwrap();
        assert_eq!(inner.len(), 1);
        assert!(inner.records()[0].starts_with("CEF:0"));
    }
}
