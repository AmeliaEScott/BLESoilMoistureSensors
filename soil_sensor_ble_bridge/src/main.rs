
use bluer;
use std::error::Error;
use tokio::time;
use uuid::Uuid;
use log::{debug, info};
use soil_sensor_common::Measurement;

use bluer::{Adapter, AdapterEvent, Address, Device, DeviceEvent, DeviceProperty, DiscoveryFilter, DiscoveryTransport};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use std::{collections::HashSet, env};


/// Only devices whose name contains this string will be tried.
const PERIPHERAL_NAME_MATCH_FILTER: &str = "HelloRust";
/// UUID of the characteristic for which we should subscribe to notifications.
const NOTIFY_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x866a5627_a761_47cc_9976_7457450e8258);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let with_changes = true;
    let all_properties = true;
    let br_edr_only = env::args().any(|arg| arg == "--bredr");
    let filter_addr: HashSet<_> = env::args().filter_map(|arg| arg.parse::<Address>().ok()).collect();

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    println!("Discovering devices using Bluetooth adapter {}\n", adapter.name());
    adapter.set_powered(true).await?;

    let filter = DiscoveryFilter {
        transport: DiscoveryTransport::Le,
        pattern: None,
        ..Default::default()
    };
    adapter.set_discovery_filter(filter).await?;
    println!("Using discovery filter:\n{:#?}\n\n", adapter.discovery_filter().await);

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    loop {
        if let Some(device_event) = device_events.next().await {
            match device_event {
                AdapterEvent::DeviceAdded(addr) => {
                    if !filter_addr.is_empty() && !filter_addr.contains(&addr) {
                        continue;
                    }

                    debug!("Device added: {addr}");
                    let device = adapter.device(addr)?;
                    tokio::spawn(watch_device(device));
                }
                AdapterEvent::DeviceRemoved(addr) => {
                    println!("Device removed: {addr}");
                }
                _ => (),
            }
        }
    }

    Ok(())
}

pub async fn watch_device(device: Device) -> bluer::Result<()> {
    let events = device.events().await?;
    pin_mut!(events);

    let name: String = loop {
        if let Some(name) = device.name().await? {
            break name;
        } else {
            events.next().await;
        }
    };

    if !name.starts_with("BLE Soil Sensor") {
        debug!("Device \"{}\" is not a soil sensor.", name);
        return Ok(())
    }

    let mut last_meas: Option<Measurement> = None;

    while let Some(event) = events.next().await {
        if let DeviceEvent::PropertyChanged(DeviceProperty::ManufacturerData(data)) = event {
            debug!("New data for {}: {:?}", name, data);
            let bytes = data.get(&65535).unwrap();
            let measurement = Measurement::from_bytes(soil_sensor_common::Serialized::try_from(bytes.as_slice()).unwrap());
            if Some(measurement) != last_meas {
                tokio::spawn(handle_measurement(measurement));
            } else {
                debug!("Duplicate measurement: {:?}", measurement);
            }
            last_meas = Some(measurement);
        }
    }

    Ok(())
}

pub async fn handle_measurement(meas: Measurement) {
    info!("Cool new measurement: {:?}", meas);
}