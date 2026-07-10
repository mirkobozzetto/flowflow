use serde::{Deserialize, Serialize};
use std::time::Instant;

/// How a pipeline step ended: `Ok` = the call ran, `Fallback` = it failed and a
/// degraded path answered instead, `Skipped` = a fast-path made the call unnecessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    Ok,
    Fallback,
    Skipped,
}

/// One observed pipeline step. Token counts are char-based estimates (`approx`)
/// until the provider exposes real usage; `model` is None for non-LLM steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceStep {
    pub step: String,
    pub model: Option<String>,
    pub tokens_in: Option<u32>,
    pub tokens_out: Option<u32>,
    pub approx: bool,
    pub latency_ms: u64,
    pub retries: u32,
    pub outcome: StepOutcome,
}

/// Per-question execution trace of the RAG pipeline. Observation only: it never
/// influences the answer. Persisted with the bot message, device-local.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RagTrace {
    pub steps: Vec<TraceStep>,
    pub total_ms: u64,
}

impl RagTrace {
    pub fn push(&mut self, step: TraceStep) {
        self.steps.push(step);
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn from_json(raw: &str) -> Option<RagTrace> {
        serde_json::from_str(raw).ok()
    }

    /// Compact label data for the UI line under the answer: (step count, seconds).
    pub fn summary(&self) -> (usize, f64) {
        (self.steps.len(), self.total_ms as f64 / 1000.0)
    }
}

/// Rough token estimate from text length; real usage plumbing can replace it later.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4) as u32
}

/// Measure one step around a call site: start it, then `finish` with what happened.
pub struct StepTimer {
    step: &'static str,
    model: Option<String>,
    started: Instant,
}

impl StepTimer {
    pub fn start(step: &'static str, model: Option<&str>) -> Self {
        Self {
            step,
            model: model.map(String::from),
            started: Instant::now(),
        }
    }

    pub fn finish(
        self,
        trace: &mut RagTrace,
        tokens_in: Option<u32>,
        tokens_out: Option<u32>,
        outcome: StepOutcome,
    ) {
        trace.push(TraceStep {
            step: self.step.to_string(),
            model: self.model,
            tokens_in,
            tokens_out,
            approx: true,
            latency_ms: self.started.elapsed().as_millis() as u64,
            retries: 0,
            outcome,
        });
    }
}
