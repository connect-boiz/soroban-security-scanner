//! Analysis job queue with priority and resource management.
//!
//! Admits jobs only while the total in-flight resource reservation stays within
//! a global budget (back-pressure against resource exhaustion), and dispatches
//! the highest-priority job first. Completing a job releases its reservation.

use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use uuid::Uuid;

/// Scheduling priority (higher runs first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Background / bulk work.
    Low,
    /// Normal interactive work.
    Normal,
    /// High priority.
    High,
    /// Critical (e.g. security re-scan).
    Critical,
}

/// A queued analysis job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisJob {
    /// Job id.
    pub id: Uuid,
    /// Scheduling priority.
    pub priority: Priority,
    /// Estimated CPU units this job will reserve.
    pub est_cpu_units: u64,
    /// Submission sequence (FIFO tie-breaker within a priority).
    seq: u64,
}

// Ordering for the max-heap: by priority, then *earlier* seq first.
impl Ord for AnalysisJob {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq)) // reverse: lower seq = higher
    }
}
impl PartialOrd for AnalysisJob {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Why a job could not be admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    /// A single job's estimate exceeds the whole global budget.
    ExceedsGlobalBudget,
    /// No capacity right now (would exceed the in-flight budget).
    AtCapacity,
}

/// Priority queue with global resource accounting.
pub struct AnalysisQueue {
    heap: BinaryHeap<AnalysisJob>,
    global_cpu_budget: u64,
    reserved: u64,
    next_seq: u64,
}

impl AnalysisQueue {
    /// Creates a queue with a global in-flight CPU reservation budget.
    pub fn new(global_cpu_budget: u64) -> Self {
        Self {
            heap: BinaryHeap::new(),
            global_cpu_budget,
            reserved: 0,
            next_seq: 0,
        }
    }

    /// Currently reserved (in-flight) CPU units.
    pub fn reserved(&self) -> u64 {
        self.reserved
    }

    /// Number of queued (not-yet-dispatched) jobs.
    pub fn pending(&self) -> usize {
        self.heap.len()
    }

    /// Enqueues a job, returning its id. Rejects jobs whose estimate alone
    /// exceeds the global budget (they could never run).
    pub fn submit(
        &mut self,
        priority: Priority,
        est_cpu_units: u64,
    ) -> Result<Uuid, AdmissionError> {
        if est_cpu_units > self.global_cpu_budget {
            return Err(AdmissionError::ExceedsGlobalBudget);
        }
        let id = Uuid::new_v4();
        let job = AnalysisJob {
            id,
            priority,
            est_cpu_units,
            seq: self.next_seq,
        };
        self.next_seq += 1;
        self.heap.push(job);
        Ok(id)
    }

    /// Dispatches the highest-priority job that fits the remaining budget,
    /// reserving its resources. Returns `Err(AtCapacity)` if the top job does
    /// not fit, or `Ok(None)` if the queue is empty.
    pub fn dispatch(&mut self) -> Result<Option<AnalysisJob>, AdmissionError> {
        let Some(top) = self.heap.peek() else {
            return Ok(None);
        };
        if self.reserved + top.est_cpu_units > self.global_cpu_budget {
            return Err(AdmissionError::AtCapacity);
        }
        let job = self.heap.pop().expect("peeked job exists");
        self.reserved += job.est_cpu_units;
        Ok(Some(job))
    }

    /// Releases a completed job's reservation, freeing capacity.
    pub fn complete(&mut self, job: &AnalysisJob) {
        self.reserved = self.reserved.saturating_sub(job.est_cpu_units);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_priority_dispatched_first() {
        let mut q = AnalysisQueue::new(1_000_000);
        q.submit(Priority::Low, 10).unwrap();
        q.submit(Priority::Critical, 10).unwrap();
        q.submit(Priority::Normal, 10).unwrap();
        assert_eq!(q.dispatch().unwrap().unwrap().priority, Priority::Critical);
        assert_eq!(q.dispatch().unwrap().unwrap().priority, Priority::Normal);
        assert_eq!(q.dispatch().unwrap().unwrap().priority, Priority::Low);
    }

    #[test]
    fn fifo_within_same_priority() {
        let mut q = AnalysisQueue::new(1_000_000);
        let first = q.submit(Priority::Normal, 10).unwrap();
        let second = q.submit(Priority::Normal, 10).unwrap();
        assert_eq!(q.dispatch().unwrap().unwrap().id, first);
        assert_eq!(q.dispatch().unwrap().unwrap().id, second);
    }

    #[test]
    fn oversized_job_rejected() {
        let mut q = AnalysisQueue::new(100);
        assert_eq!(
            q.submit(Priority::High, 200),
            Err(AdmissionError::ExceedsGlobalBudget)
        );
    }

    #[test]
    fn backpressure_at_capacity() {
        let mut q = AnalysisQueue::new(100);
        q.submit(Priority::Normal, 60).unwrap();
        q.submit(Priority::Normal, 60).unwrap();
        let a = q.dispatch().unwrap().unwrap(); // reserve 60
        assert_eq!(q.reserved(), 60);
        // Second needs 60 more but only 40 free → at capacity.
        assert_eq!(q.dispatch(), Err(AdmissionError::AtCapacity));
        // Completing the first frees capacity.
        q.complete(&a);
        assert!(q.dispatch().unwrap().is_some());
    }

    #[test]
    fn empty_queue_dispatches_none() {
        let mut q = AnalysisQueue::new(100);
        assert_eq!(q.dispatch().unwrap(), None);
    }
}
