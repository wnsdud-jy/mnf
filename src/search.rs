use anyhow::Result;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::{
    checker::NameChecker,
    generator::CandidateGenerator,
    model::{SearchEvent, SearchOptions, SearchProgress, SearchStopReason, SearchSummary},
};

pub async fn run_search<C, E>(
    options: SearchOptions,
    checker: &C,
    cancel: CancellationToken,
    mut emit: E,
) -> Result<SearchSummary>
where
    C: NameChecker,
    E: FnMut(SearchEvent) + Send,
{
    let mut generator = CandidateGenerator::new(&options);
    let mut progress = SearchProgress::default();
    emit(SearchEvent::Progress(progress.clone()));

    loop {
        if progress.found >= options.results as u64 {
            let summary = SearchSummary {
                progress,
                stop_reason: SearchStopReason::ReachedResultTarget,
            };
            emit(SearchEvent::Finished(summary.clone()));
            return Ok(summary);
        }

        if let Some(summary) = cancelled_summary(&cancel, &progress, &mut emit) {
            return Ok(summary);
        }

        if progress.checked >= options.max_checks as u64 {
            let summary = SearchSummary {
                progress,
                stop_reason: SearchStopReason::ReachedCheckBudget,
            };
            emit(SearchEvent::Finished(summary.clone()));
            return Ok(summary);
        }

        let remaining_budget = options.max_checks.saturating_sub(progress.checked as usize);
        let batch_limit = options.batch_size.min(remaining_budget);
        let batch: Vec<String> = generator.by_ref().take(batch_limit).collect();

        if batch.is_empty() {
            let summary = SearchSummary {
                progress,
                stop_reason: SearchStopReason::ExhaustedSearchSpace,
            };
            emit(SearchEvent::Finished(summary.clone()));
            return Ok(summary);
        }

        progress.generated += batch.len() as u64;
        emit(SearchEvent::Progress(progress.clone()));

        let batch_check = checker.check_batch(&batch);
        tokio::pin!(batch_check);

        let outcome = tokio::select! {
            _ = cancel.cancelled() => {
                let summary = emit_cancelled(progress, &mut emit);
                return Ok(summary);
            }
            outcome = &mut batch_check => outcome?,
        };
        progress.checked += batch.len() as u64;
        progress.batches += 1;

        for name in outcome.likely_available_names {
            if progress.found >= options.results as u64 {
                break;
            }
            progress.found += 1;
            emit(SearchEvent::Hit(name));
        }

        emit(SearchEvent::Progress(progress.clone()));

        if progress.found < options.results as u64 && !options.request_interval.is_zero() {
            tokio::select! {
                _ = cancel.cancelled() => {
                    let summary = emit_cancelled(progress, &mut emit);
                    return Ok(summary);
                }
                _ = sleep(options.request_interval) => {}
            }
        }
    }
}

fn cancelled_summary<E>(
    cancel: &CancellationToken,
    progress: &SearchProgress,
    emit: &mut E,
) -> Option<SearchSummary>
where
    E: FnMut(SearchEvent) + Send,
{
    cancel
        .is_cancelled()
        .then(|| emit_cancelled(progress.clone(), emit))
}

fn emit_cancelled<E>(progress: SearchProgress, emit: &mut E) -> SearchSummary
where
    E: FnMut(SearchEvent) + Send,
{
    let summary = SearchSummary {
        progress,
        stop_reason: SearchStopReason::Cancelled,
    };
    emit(SearchEvent::Finished(summary.clone()));
    summary
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use async_trait::async_trait;
    use std::collections::HashSet;
    use tokio::{
        sync::Notify,
        time::{Duration, timeout},
    };
    use tokio_util::sync::CancellationToken;

    use crate::{
        checker::{BatchCheckOutcome, NameChecker},
        model::{SearchEvent, SearchOptions, SearchStopReason},
        validation::validate_search_options,
    };

    use super::run_search;

    struct MockChecker {
        taken: HashSet<String>,
    }

    struct SlowChecker {
        notify: Notify,
    }

    #[async_trait]
    impl NameChecker for MockChecker {
        async fn check_batch(&self, batch: &[String]) -> Result<BatchCheckOutcome> {
            let mut taken_names = Vec::new();
            let mut likely_available_names = Vec::new();

            for name in batch {
                if self.taken.contains(name) {
                    taken_names.push(name.clone());
                } else {
                    likely_available_names.push(name.clone());
                }
            }

            Ok(BatchCheckOutcome {
                taken_names,
                likely_available_names,
            })
        }
    }

    #[async_trait]
    impl NameChecker for SlowChecker {
        async fn check_batch(&self, _batch: &[String]) -> Result<BatchCheckOutcome> {
            self.notify.notified().await;
            Ok(BatchCheckOutcome {
                taken_names: Vec::new(),
                likely_available_names: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn stops_when_target_is_reached() {
        let mut options = validate_search_options(4, "e", 3, 20).expect("valid options");
        options.request_interval = std::time::Duration::ZERO;

        let checker = MockChecker {
            taken: HashSet::from(["eaaa".to_string()]),
        };
        let mut hits = Vec::new();

        let summary = run_search(options, &checker, CancellationToken::new(), |event| {
            if let SearchEvent::Hit(name) = event {
                hits.push(name);
            }
        })
        .await
        .expect("search succeeds");

        assert_eq!(summary.stop_reason, SearchStopReason::ReachedResultTarget);
        assert_eq!(hits.len(), 3);
    }

    #[tokio::test]
    async fn stops_when_check_budget_is_hit() {
        let mut options: SearchOptions =
            validate_search_options(4, "e", 5, 2).expect("valid options");
        options.request_interval = std::time::Duration::ZERO;

        let checker = MockChecker {
            taken: HashSet::new(),
        };

        let summary = run_search(options, &checker, CancellationToken::new(), |_| {})
            .await
            .expect("search succeeds");

        assert_eq!(summary.stop_reason, SearchStopReason::ReachedCheckBudget);
        assert_eq!(summary.progress.checked, 2);
    }

    #[tokio::test]
    async fn stops_while_batch_request_is_in_flight() {
        let mut options: SearchOptions =
            validate_search_options(4, "e", 5, 20).expect("valid options");
        options.request_interval = Duration::ZERO;

        let checker = SlowChecker {
            notify: Notify::new(),
        };
        let cancel = CancellationToken::new();

        let search = run_search(options, &checker, cancel.clone(), |_| {});
        tokio::pin!(search);

        cancel.cancel();

        let summary = timeout(Duration::from_millis(50), &mut search)
            .await
            .expect("search should react to cancellation quickly")
            .expect("search succeeds");

        assert_eq!(summary.stop_reason, SearchStopReason::Cancelled);
    }
}
