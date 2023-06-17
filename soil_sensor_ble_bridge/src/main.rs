mod listener;
mod sensor_manager;

use btleplug::api::{Central, CharPropFlags, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use futures::stream::StreamExt;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

/// Only devices whose name contains this string will be tried.
const PERIPHERAL_NAME_MATCH_FILTER: &str = "HelloRust";
/// UUID of the characteristic for which we should subscribe to notifications.
const NOTIFY_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x866a5627_a761_47cc_9976_7457450e8258);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    listener::main_loop().await;

    Ok(())
}