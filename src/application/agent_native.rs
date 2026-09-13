//! Admission of explicitly declared native capabilities for new assistants.
//! This does not change legacy direct-tool behavior or enable schema 2.

use std::collections::BTreeSet;

use rig::tool::Tool;

use crate::application::tools::{
    CreateNote, ScheduleReminder, SearchNotes, SearchWeb, SummarizeFolder,
};
use crate::domain::governance::{Action, Approval, ToolPolicy};

/// Local tool identity, not a connector type, slug or OAuth dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCapability {
    SearchNotes,
    CreateNote,
    SummarizeFolder,
    SearchWeb,
    ScheduleReminder,
}

impl NativeCapability {
    pub fn name(self) -> &'static str {
        match self {
            Self::SearchNotes => SearchNotes::NAME,
            Self::CreateNote => CreateNote::NAME,
            Self::SummarizeFolder => SummarizeFolder::NAME,
            Self::SearchWeb => SearchWeb::NAME,
            Self::ScheduleReminder => ScheduleReminder::NAME,
        }
    }

    pub fn action(self) -> Action {
        match self {
            Self::SearchNotes | Self::SearchWeb => Action::Search,
            Self::SummarizeFolder => Action::Read,
            Self::CreateNote | Self::ScheduleReminder => Action::Create,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [
            Self::SearchNotes,
            Self::CreateNote,
            Self::SummarizeFolder,
            Self::SearchWeb,
            Self::ScheduleReminder,
        ]
        .into_iter()
        .find(|capability| capability.name() == name)
    }
}

/// A validated native-only policy subset. Private fields prevent construction
/// that skips admission. Runtime wiring must still await each write decision.
#[derive(Debug, Clone)]
pub struct NativeCapabilityPlan {
    policies: Vec<ToolPolicy>,
}

impl NativeCapabilityPlan {
    pub fn policies(&self) -> &[ToolPolicy] {
        &self.policies
    }

    pub fn requires_write_approval(&self) -> bool {
        self.policies.iter().any(|policy| {
            NativeCapability::from_name(&policy.tool)
                .is_some_and(|capability| capability.action().is_write())
        })
    }
}

/// `available` comes from the actual run's mounted tools, not package claims.
/// No external connector is required for a non-empty native-only plan. Exa
/// search is still a native tool but is unavailable when not mounted/configured.
/// A bool here records admission availability, not delivery of a live decision.
pub fn validate_native_capabilities(
    policies: &[ToolPolicy],
    available: &[NativeCapability],
    approval_channel_available: bool,
) -> Result<NativeCapabilityPlan, String> {
    let mut seen = BTreeSet::new();
    for policy in policies {
        let capability =
            NativeCapability::from_name(&policy.tool).ok_or_else(|| {
                format!("unsupported native capability `{}`", policy.tool)
            })?;
        if !seen.insert(policy.tool.as_str()) {
            return Err(format!(
                "duplicate native capability `{}`",
                policy.tool
            ));
        }
        if !available.contains(&capability) {
            return Err(format!(
                "native capability `{}` is not available in this run",
                policy.tool
            ));
        }
        if !policy.mode.allows(capability.action()) {
            return Err(format!(
                "native capability `{}` has an incompatible mode",
                policy.tool
            ));
        }
        if capability.action().is_write()
            && (policy.approval != Approval::RequireApproval
                || !approval_channel_available)
        {
            return Err(format!("native write `{}` requires user approval and a decision channel", policy.tool));
        }
        // Native tools do not implement spreadsheet upsert or column semantics.
        if policy.key_columns.is_some() {
            return Err(format!(
                "native capability `{}` cannot use key_columns",
                policy.tool
            ));
        }
    }
    Ok(NativeCapabilityPlan {
        policies: policies.to_vec(),
    })
}
