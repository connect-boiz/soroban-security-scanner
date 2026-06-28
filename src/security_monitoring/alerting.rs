//! Real-time, multi-channel security alerting.
//!
//! Routes incidents to notification channels (email, SMS, Slack, PagerDuty)
//! based on priority: higher-priority incidents fan out to more (and more
//! intrusive) channels. Channels are pluggable via [`NotificationChannel`]; a
//! recording double is provided for tests.

use crate::security_monitoring::incident::{Incident, Priority};
use serde::{Deserialize, Serialize};

/// A delivery channel kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelKind {
    /// Email.
    Email,
    /// SMS.
    Sms,
    /// Slack.
    Slack,
    /// PagerDuty.
    PagerDuty,
}

/// A rendered alert message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertMessage {
    /// Incident subject.
    pub subject: String,
    /// Priority.
    pub priority: Priority,
    /// Short title.
    pub title: String,
    /// Body text.
    pub body: String,
}

impl AlertMessage {
    /// Renders an alert from an incident.
    pub fn from_incident(incident: &Incident) -> Self {
        Self {
            subject: incident.subject.clone(),
            priority: incident.priority(),
            title: format!(
                "[{:?}] security incident: {}",
                incident.priority(),
                incident.subject
            ),
            body: format!(
                "severity={} findings={} rules={}",
                incident.severity.as_str(),
                incident.finding_count,
                incident.rules.join(",")
            ),
        }
    }
}

/// A pluggable notification channel.
pub trait NotificationChannel: Send + Sync {
    /// The kind of channel.
    fn kind(&self) -> ChannelKind;
    /// Delivers a message. Returns `Ok(())` on accepted delivery.
    fn deliver(&self, message: &AlertMessage) -> Result<(), String>;
}

/// Routing policy: which channels fire at each priority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertRouting {
    /// Channels used for P1 (most intrusive).
    pub p1: Vec<ChannelKind>,
    /// Channels used for P2.
    pub p2: Vec<ChannelKind>,
    /// Channels used for P3.
    pub p3: Vec<ChannelKind>,
    /// Channels used for P4.
    pub p4: Vec<ChannelKind>,
}

impl Default for AlertRouting {
    fn default() -> Self {
        Self {
            p1: vec![
                ChannelKind::PagerDuty,
                ChannelKind::Slack,
                ChannelKind::Email,
                ChannelKind::Sms,
            ],
            p2: vec![ChannelKind::Slack, ChannelKind::Email],
            p3: vec![ChannelKind::Email],
            p4: vec![],
        }
    }
}

impl AlertRouting {
    /// Channels to notify for the given priority.
    pub fn channels_for(&self, priority: Priority) -> &[ChannelKind] {
        match priority {
            Priority::P1 => &self.p1,
            Priority::P2 => &self.p2,
            Priority::P3 => &self.p3,
            Priority::P4 => &self.p4,
        }
    }
}

/// The outcome of dispatching an alert across channels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchResult {
    /// Channels that accepted the message.
    pub delivered: Vec<ChannelKind>,
    /// Channels that failed, with error text.
    pub failed: Vec<(ChannelKind, String)>,
}

impl DispatchResult {
    /// Whether at least one channel accepted delivery.
    pub fn any_delivered(&self) -> bool {
        !self.delivered.is_empty()
    }
}

/// Dispatches alerts to registered channels per the routing policy.
pub struct AlertDispatcher {
    routing: AlertRouting,
    channels: Vec<Box<dyn NotificationChannel>>,
}

impl AlertDispatcher {
    /// Creates a dispatcher.
    pub fn new(routing: AlertRouting) -> Self {
        Self {
            routing,
            channels: Vec::new(),
        }
    }

    /// Registers a channel.
    pub fn register(&mut self, channel: Box<dyn NotificationChannel>) {
        self.channels.push(channel);
    }

    /// Dispatches an alert for an incident across its routed channels.
    pub fn dispatch(&self, incident: &Incident) -> DispatchResult {
        let message = AlertMessage::from_incident(incident);
        let wanted = self.routing.channels_for(incident.priority());
        let mut delivered = Vec::new();
        let mut failed = Vec::new();

        for kind in wanted {
            for channel in self.channels.iter().filter(|c| c.kind() == *kind) {
                match channel.deliver(&message) {
                    Ok(()) => delivered.push(*kind),
                    Err(e) => failed.push((*kind, e)),
                }
            }
        }
        DispatchResult { delivered, failed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_monitoring::detection::Finding;
    use crate::security_monitoring::event::SecuritySeverity;
    use std::sync::Mutex;

    struct RecordingChannel {
        kind: ChannelKind,
        sent: Mutex<Vec<AlertMessage>>,
        fail: bool,
    }

    impl RecordingChannel {
        fn new(kind: ChannelKind) -> Self {
            Self {
                kind,
                sent: Mutex::new(Vec::new()),
                fail: false,
            }
        }
    }

    impl NotificationChannel for RecordingChannel {
        fn kind(&self) -> ChannelKind {
            self.kind
        }
        fn deliver(&self, message: &AlertMessage) -> Result<(), String> {
            if self.fail {
                return Err("delivery failed".to_string());
            }
            self.sent.lock().unwrap().push(message.clone());
            Ok(())
        }
    }

    fn incident(sev: SecuritySeverity) -> Incident {
        Incident::open(
            &Finding {
                rule: "r".to_string(),
                subject: "alice".to_string(),
                severity: sev,
                detail: "d".to_string(),
                at: 1000,
            },
            1000,
            1010,
        )
    }

    #[test]
    fn p1_fans_out_to_all_channels() {
        let mut d = AlertDispatcher::new(AlertRouting::default());
        d.register(Box::new(RecordingChannel::new(ChannelKind::PagerDuty)));
        d.register(Box::new(RecordingChannel::new(ChannelKind::Slack)));
        d.register(Box::new(RecordingChannel::new(ChannelKind::Email)));
        d.register(Box::new(RecordingChannel::new(ChannelKind::Sms)));

        let result = d.dispatch(&incident(SecuritySeverity::Critical)); // P1
        assert_eq!(result.delivered.len(), 4);
        assert!(result.delivered.contains(&ChannelKind::PagerDuty));
    }

    #[test]
    fn medium_only_emails() {
        let mut d = AlertDispatcher::new(AlertRouting::default());
        d.register(Box::new(RecordingChannel::new(ChannelKind::Email)));
        d.register(Box::new(RecordingChannel::new(ChannelKind::PagerDuty)));
        let result = d.dispatch(&incident(SecuritySeverity::Medium)); // P3 → email only
        assert_eq!(result.delivered, vec![ChannelKind::Email]);
    }

    #[test]
    fn failed_channel_is_recorded() {
        let mut d = AlertDispatcher::new(AlertRouting::default());
        d.register(Box::new(RecordingChannel {
            kind: ChannelKind::Email,
            sent: Mutex::new(Vec::new()),
            fail: true,
        }));
        let result = d.dispatch(&incident(SecuritySeverity::Medium));
        assert!(!result.any_delivered());
        assert_eq!(result.failed.len(), 1);
    }

    #[test]
    fn p4_notifies_nothing() {
        let mut d = AlertDispatcher::new(AlertRouting::default());
        d.register(Box::new(RecordingChannel::new(ChannelKind::Email)));
        let result = d.dispatch(&incident(SecuritySeverity::Low)); // P4
        assert!(!result.any_delivered());
    }

    #[test]
    fn message_renders_incident_detail() {
        let msg = AlertMessage::from_incident(&incident(SecuritySeverity::Critical));
        assert!(msg.title.contains("P1"));
        assert!(msg.body.contains("CRITICAL"));
    }
}
