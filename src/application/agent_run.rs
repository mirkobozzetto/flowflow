//! Shared accounting and stop state for opt-in scoped/native execution.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rig::tool::Tool;
use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

use crate::application::agent_native::NativeCapabilityPlan;
use crate::application::agent_native_execution::execute_native_guarded;
use crate::application::tools::{ContractHook, ToolEvent, ToolFailure};
use crate::domain::governance::{
    ConnectorManifest, Governance, Limits, RunState, ToolPolicy,
};

pub struct ScopedAgentRun {
    hook: ContractHook,
    native: NativeCapabilityPlan,
    state: Arc<Mutex<RunState>>,
    stopped: AtomicBool,
    limits: Limits,
    events: UnboundedSender<ToolEvent>,
    clock: Mutex<RunClock>,
    native_log: Mutex<Vec<String>>,
    admission: Option<Arc<dyn Fn() -> Result<(), String> + Send + Sync>>,
}

impl ScopedAgentRun {
    pub fn new(
        events: UnboundedSender<ToolEvent>,
        mut external: Vec<(String, Governance, ConnectorManifest)>,
        native: NativeCapabilityPlan,
        limits: Limits,
    ) -> Self {
        let state = Arc::default();
        for (_, governance, _) in &mut external {
            governance.limits = Some(limits.clone());
        }
        Self {
            hook: ContractHook::with_shared_run(
                events.clone(),
                external,
                Arc::clone(&state),
            ),
            native,
            state,
            stopped: AtomicBool::new(false),
            limits,
            events,
            clock: Mutex::new(RunClock {
                start: Instant::now(),
                held: Duration::ZERO,
                since: None,
                waiters: 0,
            }),
            native_log: Mutex::new(Vec::new()),
            admission: None,
        }
    }

    pub fn native_tool_names(&self) -> Vec<&str> {
        self.native
            .policies()
            .iter()
            .map(|policy| policy.tool.as_str())
            .collect()
    }

    /// Recheck a pinned installation before proposing and after approval.
    pub fn with_admission_check(
        mut self,
        check: Arc<dyn Fn() -> Result<(), String> + Send + Sync>,
    ) -> Self {
        self.hook = self.hook.with_admission_check(check.clone());
        self.admission = Some(check);
        self
    }

    pub fn elapsed_seconds(&self) -> u64 {
        let clock = self.clock.lock().expect("run clock poisoned");
        let held = clock.held
            + clock.since.map(|since| since.elapsed()).unwrap_or_default();
        clock
            .start
            .elapsed()
            .saturating_sub(held)
            .as_secs()
            .saturating_sub(self.hook.held_seconds())
    }

    pub fn drain_native_events(&self) -> Vec<String> {
        std::mem::take(
            &mut *self.native_log.lock().expect("native log poisoned"),
        )
    }

    fn pause_clock(&self) -> ApprovalWait<'_> {
        let mut clock = self.clock.lock().expect("run clock poisoned");
        if clock.waiters == 0 {
            clock.since = Some(Instant::now());
        }
        clock.waiters += 1;
        ApprovalWait(self)
    }

    /// Only external calls go through this hook. Declared native tools must use
    /// execute_native; mounting raw native tools would retain the legacy bypass.
    pub fn with_external_context(
        mut self,
        peers: Vec<(String, rmcp::service::ServerSink)>,
        schema: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        self.hook = self.hook.with_peers(peers).with_schema(schema);
        self
    }

    pub fn external_hook(&self) -> &ContractHook {
        &self.hook
    }

    pub fn aborted(&self) -> bool {
        self.stopped.load(Ordering::Relaxed) || self.hook.aborted()
    }

    pub fn abort(&self) {
        self.stopped.store(true, Ordering::Relaxed);
        self.hook.abort_run();
    }

    /// The existing chain runtime owns progress and excludes approval wait time.
    /// This also works for native-only runs, which have no first connector.
    pub fn set_progress(&self, steps: u32, elapsed_seconds: u64) {
        let mut state = self.state.lock().expect("run state poisoned");
        state.steps = state.steps.max(steps);
        state.elapsed_seconds = state.elapsed_seconds.max(elapsed_seconds);
    }

    fn check_native(
        &self,
        policy: &ToolPolicy,
        charge: bool,
    ) -> Result<(), ToolFailure> {
        if let Some(check) = &self.admission {
            check().map_err(ToolFailure)?;
        }
        let elapsed = self.elapsed_seconds();
        let mut state = self.state.lock().expect("run state poisoned");
        state.elapsed_seconds = state.elapsed_seconds.max(elapsed);
        if self.aborted() {
            return Err(ToolFailure("run actions have ended".into()));
        }
        if policy.max_calls_per_run.is_some_and(|max| {
            state.per_tool.get(&policy.tool).copied().unwrap_or(0) >= max
        }) {
            return Err(ToolFailure(
                "native per-tool call budget reached".into(),
            ));
        }
        if self
            .limits
            .max_tool_calls
            .is_some_and(|max| state.tool_calls >= max)
        {
            return Err(ToolFailure("shared tool-call budget reached".into()));
        }
        if self.limits.max_steps.is_some_and(|max| state.steps >= max) {
            return Err(ToolFailure("shared step budget reached".into()));
        }
        if self
            .limits
            .max_run_seconds
            .is_some_and(|max| state.elapsed_seconds >= u64::from(max))
        {
            return Err(ToolFailure("shared run time budget reached".into()));
        }
        if charge {
            state.tool_calls = state.tool_calls.saturating_add(1);
            let count = state.per_tool.entry(policy.tool.clone()).or_default();
            *count = count.saturating_add(1);
        }
        Ok(())
    }

    pub async fn execute_native<T: Tool>(
        &self,
        tool: &T,
        args: Value,
        timeout: Duration,
    ) -> Result<T::Output, ToolFailure> {
        let policy = self
            .native
            .policies()
            .iter()
            .find(|policy| policy.tool == T::NAME)
            .ok_or_else(|| {
                ToolFailure("native tool was not admitted".into())
            })?;
        let mut attempt = AbortIncompleteCall {
            run: self,
            completed: false,
        };
        let result = execute_native_guarded(
            tool,
            &self.native,
            args,
            &self.events,
            timeout,
            |charge| self.check_native(policy, charge),
            || self.pause_clock(),
        )
        .await;
        // Fail closed across owners, including expiry, rejected/invalid edits,
        // closed surfaces and a budget exhausted while a card was pending.
        attempt.completed = result.is_ok();
        let detail = match &result {
            Ok(output) => serde_json::to_string(output)
                .unwrap_or_else(|_| "completed".into()),
            Err(error) => format!("refused or failed: {error}"),
        };
        self.native_log
            .lock()
            .expect("native log poisoned")
            .push(format!("{}: {detail}", T::NAME));
        result
    }
}

struct AbortIncompleteCall<'a> {
    run: &'a ScopedAgentRun,
    completed: bool,
}

impl Drop for AbortIncompleteCall<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.run.abort();
        }
    }
}

// Count overlapping approval waits once. Dropped/cancelled calls also resume
// the clock; actual tool execution is never inside this pause.
struct RunClock {
    start: Instant,
    held: Duration,
    since: Option<Instant>,
    waiters: u32,
}
struct ApprovalWait<'a>(&'a ScopedAgentRun);
impl Drop for ApprovalWait<'_> {
    fn drop(&mut self) {
        let mut clock = self.0.clock.lock().expect("run clock poisoned");
        clock.waiters -= 1;
        if clock.waiters == 0 {
            if let Some(since) = clock.since.take() {
                clock.held += since.elapsed();
            }
        }
    }
}
