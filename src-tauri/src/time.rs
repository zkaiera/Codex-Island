use chrono::{DateTime, Utc};

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}
