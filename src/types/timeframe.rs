use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    M30,
    H1,
    H4,
    D1,
}

impl Timeframe {
    pub const fn window_minutes(&self) -> usize {
        match self {
            Timeframe::M1 => 1,
            Timeframe::M5 => 5,
            Timeframe::M15 => 15,
            Timeframe::M30 => 30,
            Timeframe::H1 => 60,
            Timeframe::H4 => 4 * 60,
            Timeframe::D1 => 24 * 60,
        }
    }
    pub const fn window_millis(&self) -> i64 {
        match self {
            Timeframe::M1 => 60_000,
            Timeframe::M5 => 5 * 60_000,
            Timeframe::M15 => 15 * 60_000,
            Timeframe::M30 => 30 * 60_000,
            Timeframe::H1 => 60 * 60_000,
            Timeframe::H4 => 4 * 60 * 60_000,
            Timeframe::D1 => 24 * 60 * 60_000,
        }
    }

    pub const fn to_string(&self) -> &str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::M30 => "30m",
            Timeframe::H1 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::D1 => "1d",
        }
    }

    pub const fn nearest_ms(&self, now_ms: i64) -> i64 {
        let window = self.window_millis();
        now_ms - (now_ms % window)
    }
}

#[cfg(test)]
mod tests {
    use super::Timeframe;

    #[test]
    fn nearest_ms_floors_to_window() {
        // 13:56:05 UTC expressed in milliseconds from midnight.
        let ts = (13 * 3_600_000) + (56 * 60_000) + 5_000;

        assert_eq!(
            Timeframe::M1.nearest_ms(ts),
            (13 * 3_600_000) + (56 * 60_000)
        );
        assert_eq!(
            Timeframe::M5.nearest_ms(ts),
            (13 * 3_600_000) + (55 * 60_000)
        );
        assert_eq!(
            Timeframe::M15.nearest_ms(ts),
            (13 * 3_600_000) + (45 * 60_000)
        );
        assert_eq!(
            Timeframe::M30.nearest_ms(ts),
            (13 * 3_600_000) + (30 * 60_000)
        );
        assert_eq!(Timeframe::H1.nearest_ms(ts), 13 * 3_600_000);
    }
}
