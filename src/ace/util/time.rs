pub struct Time;

impl Time {
    pub fn unix_timestamp_millis(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }
}

pub fn now() -> Time {
    Time
}
