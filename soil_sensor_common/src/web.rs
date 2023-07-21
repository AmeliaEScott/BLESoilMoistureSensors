use crate::Measurement;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Copy, Clone)]
pub struct Request {
    pub measurement: Measurement,
    #[serde(with = "time::serde::rfc2822")]
    pub timestamp: time::OffsetDateTime,
    pub sensor_address: [u8; 6]
}
