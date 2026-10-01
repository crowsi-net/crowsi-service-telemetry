use crate::FreshnessState;

pub fn freshness(now_ms: i64, observed: i64, valid_for: i64) -> FreshnessState {
    if observed <= 0 || valid_for <= 0 {
        return FreshnessState::Unknown;
    }
    let valid_until = observed.saturating_add(valid_for);
    if now_ms <= valid_until {
        FreshnessState::Fresh
    } else if now_ms <= valid_until.saturating_add(valid_for) {
        FreshnessState::Degraded
    } else {
        FreshnessState::Stale
    }
}

pub fn phase(value: FreshnessState) -> i64 {
    match value {
        FreshnessState::Fresh | FreshnessState::Unknown => 0,
        FreshnessState::Degraded => 1,
        FreshnessState::Stale => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_changes_without_a_storage_revision() {
        assert_eq!(freshness(10_500, 10_000, 1_000), FreshnessState::Fresh);
        assert_eq!(freshness(11_500, 10_000, 1_000), FreshnessState::Degraded);
        assert_eq!(freshness(12_500, 10_000, 1_000), FreshnessState::Stale);
        assert_eq!(freshness(12_500, 0, 1_000), FreshnessState::Unknown);
    }
}
