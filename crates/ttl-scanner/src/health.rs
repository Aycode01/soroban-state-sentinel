//! Health-band classification of entries by ledgers remaining until archival.
//!
//! Default bands (all configurable):
//!
//! - `Healthy` — more than `healthy_min_ledgers` ledgers remaining.
//! - `ExpiringSoon` — more than `critical_max_ledgers`, at most
//!   `healthy_min_ledgers`.
//! - `Critical` — at most `critical_max_ledgers` ledgers remaining (including 0:
//!   the entry is live only through the current ledger).
//! - `Archived` — the entry is not readable in the live state at all and needs
//!   restoration.

/// Health of a ledger entry with respect to state archival.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthBand {
    /// More than `healthy_min_ledgers` remaining — no action needed.
    Healthy,
    /// Between `critical_max_ledgers` and `healthy_min_ledgers` remaining —
    /// schedule an extension.
    ExpiringSoon,
    /// `critical_max_ledgers` or fewer remaining — extend now.
    Critical,
    /// Entry is not present in the live state — restore.
    Archived,
}

impl HealthBand {
    /// Stable machine-readable name (used in the JSON schema and by repo 3).
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthBand::Healthy => "healthy",
            HealthBand::ExpiringSoon => "expiring_soon",
            HealthBand::Critical => "critical",
            HealthBand::Archived => "archived",
        }
    }

    /// True when this band should fail a scan run with `--fail-on-critical`.
    pub fn is_action_required(&self) -> bool {
        matches!(self, HealthBand::Critical | HealthBand::Archived)
    }
}

/// Band boundaries in units of ledgers.
///
/// # Units
///
/// Both fields are in **ledgers** (not seconds/days). Days are converted to
/// ledgers when the config is built from day thresholds, using the network's
/// average ledger close time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthConfig {
    /// Entries with strictly more ledgers remaining than this are `Healthy`.
    pub healthy_min_ledgers: u32,
    /// Entries with at most this many ledgers remaining are `Critical`.
    pub critical_max_ledgers: u32,
}

/// Seconds in a day — a civil-time constant, not a protocol parameter.
const SECONDS_PER_DAY: u64 = 86_400;

/// Result of converting a day threshold into a ledger threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaysConversion {
    /// The threshold converted cleanly.
    Ledgers(u32),
    /// The day threshold exceeds u32::MAX ledgers; treat as unbounded.
    Overflow,
}

/// Convert a threshold in days to ledgers, rounding up so a "> N days" band does
/// not start early.
///
/// # Units
///
/// `days` is in days, `ledger_close_seconds` in seconds per ledger. Returns
/// `None` when `ledger_close_seconds` is 0 (an invalid configuration).
pub fn days_to_ledgers(days: u32, ledger_close_seconds: u64) -> Option<DaysConversion> {
    if ledger_close_seconds == 0 {
        return None;
    }
    let day_seconds = u64::from(days).checked_mul(SECONDS_PER_DAY)?;
    let ledgers = day_seconds.div_ceil(ledger_close_seconds);
    match u32::try_from(ledgers) {
        Ok(l) => Some(DaysConversion::Ledgers(l)),
        Err(_) => Some(DaysConversion::Overflow),
    }
}

impl HealthConfig {
    /// Build band boundaries from day thresholds.
    ///
    /// # Units
    ///
    /// `healthy_min_days` / `critical_max_days` in days,
    /// `ledger_close_seconds` in seconds per ledger.
    pub fn from_days(
        healthy_min_days: u32,
        critical_max_days: u32,
        ledger_close_seconds: u64,
    ) -> Result<Self, String> {
        if ledger_close_seconds == 0 {
            return Err("ledger_close_seconds must be > 0".to_string());
        }
        let healthy = match days_to_ledgers(healthy_min_days, ledger_close_seconds)
            .ok_or_else(|| "ledger_close_seconds must be > 0".to_string())?
        {
            DaysConversion::Ledgers(l) => l,
            DaysConversion::Overflow => u32::MAX,
        };
        let critical = match days_to_ledgers(critical_max_days, ledger_close_seconds)
            .ok_or_else(|| "ledger_close_seconds must be > 0".to_string())?
        {
            DaysConversion::Ledgers(l) => l,
            DaysConversion::Overflow => u32::MAX,
        };
        if healthy < critical {
            return Err(format!(
                "healthy_min ({healthy} ledgers) must be >= critical_max ({critical} ledgers)"
            ));
        }
        Ok(Self {
            healthy_min_ledgers: healthy,
            critical_max_ledgers: critical,
        })
    }

    /// Build band boundaries directly from ledger thresholds.
    pub fn from_ledgers(healthy_min_ledgers: u32, critical_max_ledgers: u32) -> Self {
        Self {
            healthy_min_ledgers,
            critical_max_ledgers,
        }
    }
}

/// Classify an entry by its ledgers remaining.
///
/// # Units
///
/// `ledgers_remaining` is in ledgers; `None` means the entry is not readable in
/// the live state (archived).
pub fn classify(ledgers_remaining: Option<u32>, config: &HealthConfig) -> HealthBand {
    match ledgers_remaining {
        None => HealthBand::Archived,
        Some(remaining) if remaining > config.healthy_min_ledgers => HealthBand::Healthy,
        Some(remaining) if remaining > config.critical_max_ledgers => HealthBand::ExpiringSoon,
        Some(_) => HealthBand::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> HealthConfig {
        // 5s ledgers: 30 days = 518400 ledgers, 7 days = 120960 ledgers.
        HealthConfig::from_days(30, 7, 5).expect("valid config")
    }

    #[test]
    fn day_thresholds_convert_to_expected_ledgers() {
        let c = config();
        assert_eq!(c.healthy_min_ledgers, 30 * 86_400 / 5);
        assert_eq!(c.critical_max_ledgers, 7 * 86_400 / 5);
    }

    #[test]
    fn healthy_band() {
        let c = config();
        assert_eq!(
            classify(Some(c.healthy_min_ledgers + 1), &c),
            HealthBand::Healthy
        );
        assert_eq!(
            classify(Some(u32::MAX), &c),
            HealthBand::Healthy,
            "huge remaining must not overflow"
        );
    }

    #[test]
    fn expiring_soon_band_boundaries() {
        let c = config();
        assert_eq!(
            classify(Some(c.healthy_min_ledgers), &c),
            HealthBand::ExpiringSoon,
            "exactly 30 days is ExpiringSoon, not Healthy"
        );
        assert_eq!(
            classify(Some(c.critical_max_ledgers + 1), &c),
            HealthBand::ExpiringSoon
        );
    }

    #[test]
    fn critical_band_includes_zero() {
        let c = config();
        assert_eq!(
            classify(Some(c.critical_max_ledgers), &c),
            HealthBand::Critical
        );
        assert_eq!(classify(Some(1), &c), HealthBand::Critical);
        assert_eq!(
            classify(Some(0), &c),
            HealthBand::Critical,
            "0 remaining (live only through current ledger) is Critical"
        );
    }

    #[test]
    fn archived_band_for_unreadable() {
        let c = config();
        assert_eq!(classify(None, &c), HealthBand::Archived);
    }

    #[test]
    fn band_names_are_stable() {
        assert_eq!(HealthBand::Healthy.as_str(), "healthy");
        assert_eq!(HealthBand::ExpiringSoon.as_str(), "expiring_soon");
        assert_eq!(HealthBand::Critical.as_str(), "critical");
        assert_eq!(HealthBand::Archived.as_str(), "archived");
    }

    #[test]
    fn action_required_only_for_critical_and_archived() {
        assert!(!HealthBand::Healthy.is_action_required());
        assert!(!HealthBand::ExpiringSoon.is_action_required());
        assert!(HealthBand::Critical.is_action_required());
        assert!(HealthBand::Archived.is_action_required());
    }

    #[test]
    fn rejects_zero_ledger_close_time() {
        assert!(HealthConfig::from_days(30, 7, 0).is_err());
    }
}
