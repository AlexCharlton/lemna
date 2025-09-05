/// A no-std Instant
use core::ops::{Add, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    ticks_us: u64,
}

impl Instant {
    #[cfg(feature = "std")]
    pub fn now() -> Self {
        Self {
            ticks_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
        }
    }

    #[cfg(not(feature = "std"))]
    pub fn now() -> Self {
        todo!()
    }

    pub fn elapsed(self) -> Duration {
        self - Self::now()
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, other: Instant) -> Duration {
        Duration {
            ticks_us: self.ticks_us.saturating_sub(other.ticks_us) as i64,
        }
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, other: Duration) -> Instant {
        Instant {
            ticks_us: (self.ticks_us as i64 - other.ticks_us) as u64,
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, other: Duration) -> Instant {
        Instant {
            ticks_us: (self.ticks_us as i64 + other.ticks_us) as u64,
        }
    }
}

pub struct Duration {
    ticks_us: i64,
}

impl Duration {
    pub fn as_millis(&self) -> i64 {
        self.ticks_us / 1000
    }

    pub fn as_micros(&self) -> i64 {
        self.ticks_us
    }

    pub fn as_seconds(&self) -> i64 {
        self.ticks_us / 1000000
    }
}

impl Add<Duration> for Duration {
    type Output = Duration;
    fn add(self, other: Duration) -> Duration {
        Duration {
            ticks_us: self.ticks_us + other.ticks_us,
        }
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;
    fn sub(self, other: Duration) -> Duration {
        Duration {
            ticks_us: self.ticks_us - other.ticks_us,
        }
    }
}
