use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

pub(crate) fn now_seconds() -> u64 {
    now_ms() / 1_000
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenActivity {
    pub output_tokens_per_minute: u32,
    pub observed_at_ms: u64,
    pub measurement: TokenMeasurement,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenMeasurement {
    Estimated,
    Reported,
}

impl TokenActivity {
    pub fn is_fresh(&self, now_ms: u64) -> bool {
        self.output_tokens_per_minute <= 1_000_000
            && self.observed_at_ms > 0
            && self.observed_at_ms <= now_ms.saturating_add(1_000)
            && now_ms.saturating_sub(self.observed_at_ms) <= 6_000
    }

    pub(crate) fn if_fresh(self) -> Option<Self> {
        self.is_fresh(now_ms()).then_some(self)
    }
}
