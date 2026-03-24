use std::time::Duration;

pub const MIN_NAME_LENGTH: u8 = 3;
pub const MAX_NAME_LENGTH: u8 = 10;
pub const DEFAULT_RESULTS: usize = 20;
pub const DEFAULT_MAX_CHECKS: usize = 500;
pub const DEFAULT_BATCH_SIZE: usize = 10;
pub const DEFAULT_REQUEST_INTERVAL_MS: u64 = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchOptions {
    pub length: u8,
    pub prefix: String,
    pub results: usize,
    pub max_checks: usize,
    pub batch_size: usize,
    pub request_interval: Duration,
}

impl SearchOptions {
    pub fn remaining_len(&self) -> usize {
        usize::from(self.length) - self.prefix.chars().count()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchProgress {
    pub generated: u64,
    pub checked: u64,
    pub found: u64,
    pub batches: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchStopReason {
    ReachedResultTarget,
    ReachedCheckBudget,
    ExhaustedSearchSpace,
    Cancelled,
}

impl SearchStopReason {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ReachedResultTarget => "target reached",
            Self::ReachedCheckBudget => "check budget reached",
            Self::ExhaustedSearchSpace => "search space exhausted",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchSummary {
    pub progress: SearchProgress,
    pub stop_reason: SearchStopReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchEvent {
    Progress(SearchProgress),
    Hit(String),
    Finished(SearchSummary),
}
