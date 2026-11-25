//! Pure scheduling rules for optional background model loading.
const RECOVERY_MS: u64 = 30_000;
const RETRY_MS: u64 = 120_000;

#[derive(Default)]
pub struct WarmupSchedule {
    healthy_since: Option<u64>,
    recovering: bool,
    next_attempt: u64,
}

impl WarmupSchedule {
    pub fn ready(&mut self, now: u64, healthy: bool, enabled: bool, idle: bool) -> bool {
        if !healthy {
            self.healthy_since = None;
            self.recovering = true;
            self.next_attempt = now.saturating_add(RETRY_MS);
            return false;
        }
        let since = *self.healthy_since.get_or_insert(now);
        enabled
            && idle
            && now >= self.next_attempt
            && (!self.recovering || now.saturating_sub(since) >= RECOVERY_MS)
    }

    pub fn attempted(&mut self, now: u64) {
        self.next_attempt = now.saturating_add(RETRY_MS);
    }
}

/// Reserve working memory beyond the weights, plus at least 2 GiB/12% for
/// the rest of the system. Unknown memory status must not trigger a preload.
pub fn has_headroom(available_mb: u64, total_mb: u64, model_mb: u64) -> bool {
    total_mb > 0
        && available_mb.saturating_sub(model_mb.saturating_add(512))
            >= 2048.max(total_mb.saturating_mul(12) / 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_preloads_but_busy_and_disabled_do_not() {
        let mut schedule = WarmupSchedule::default();
        assert!(!schedule.ready(5_000, true, false, true));
        assert!(!schedule.ready(5_000, true, true, false));
        assert!(schedule.ready(5_000, true, true, true));
    }

    #[test]
    fn pressure_recovery_requires_stability_and_cooldown() {
        let mut schedule = WarmupSchedule::default();
        assert!(!schedule.ready(10_000, false, true, true));
        assert!(!schedule.ready(100_000, true, true, true));
        assert!(!schedule.ready(129_999, true, true, true));
        assert!(schedule.ready(130_000, true, true, true));
        assert!(!schedule.ready(140_000, false, true, true));
        assert!(!schedule.ready(260_000, true, true, true));
        assert!(schedule.ready(290_000, true, true, true));
    }

    #[test]
    fn failed_or_successful_attempts_are_rate_limited() {
        let mut schedule = WarmupSchedule::default();
        schedule.attempted(5_000);
        assert!(!schedule.ready(124_999, true, true, true));
        assert!(schedule.ready(125_000, true, true, true));
    }

    #[test]
    fn preload_reserves_space_for_weights_runtime_and_other_apps() {
        assert!(has_headroom(8_000, 16_072, 2_100));
        assert!(!has_headroom(3_000, 16_072, 2_100));
        assert!(!has_headroom(274, 16_072, 478));
        assert!(!has_headroom(8_000, 0, 478));
        assert!(!has_headroom(5_000, 64_000, 478));
    }
}
