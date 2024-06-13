use std::time::Duration;

pub const UPS: u32 = 60;
pub const UPDATE_TIME: Duration = Duration::from_millis(1000 / UPS as u64);
