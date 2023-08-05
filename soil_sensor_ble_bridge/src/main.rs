#![feature(try_blocks)]

use bluer;
use log::{debug, info, warn};
use bluer::{AdapterEvent, Address, Device, DeviceEvent, DeviceProperty, DiscoveryFilter, DiscoveryTransport};
use futures::{pin_mut, StreamExt};
use futures::future::{select_all, FutureExt};
use tokio::task::JoinHandle;
use soil_sensor_common::Measurement;
use soil_sensor_common::web::Request as MeasurementRequest;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    pretty_env_logger::init();

    let session = bluer::Session::new().await.unwrap();
    let adapter_names = session.adapter_names().await.unwrap();
    let mut adapter_tasks: Vec<JoinHandle<bluer::Result<()>>> = adapter_names
        .iter()
        .filter_map(|adapter_name|{
            if let Ok(adapter) = session.adapter(adapter_name) {
                Some(tokio::spawn(listen_adapter(adapter)))
            } else {
                warn!("Failed to create adapter {}", adapter_name);
                None
            }
        }).collect();

    while adapter_tasks.len() > 0 {
        let (result, _, remaining_tasks) =
            select_all(adapter_tasks).await;
        warn!("Result from adapter: {:?}", result);
        adapter_tasks = remaining_tasks;
    }

    info!("All adapter listener tasks wrapped up. Exiting gracefully.");
}

async fn listen_adapter(adapter: bluer::Adapter) -> bluer::Result<()> {
    debug!("Discovering devices using Bluetooth adapter {}\n", adapter.name());
    adapter.set_powered(true).await?;

    let filter = DiscoveryFilter {
        transport: DiscoveryTransport::Le,
        ..Default::default()
    };
    adapter.set_discovery_filter(filter).await?;
    debug!("Using discovery filter:\n{:#?}\n\n", adapter.discovery_filter().await);

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    loop {
        debug!("Waiting for device event...");
        let event = device_events.next().await;
        if let Some(AdapterEvent::DeviceAdded(addr)) = event {
            debug!("Device added: {addr}");
            let device = adapter.device(addr)?;
            tokio::spawn(watch_device(device));
        } else {
            debug!("Device Event: {:?}", event);
        }
    }
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
            let result: Result<(), String> = try {
                let id = u16::from_be_bytes(soil_sensor_common::COMPANY_ID_CODE);
                let bytes = data.get(&id).ok_or(format!("Data {:?} has no key {}", data, id))?;
                let bytes = soil_sensor_common::Serialized::try_from(
                    bytes.as_slice())
                    .or(Err(format!("Error converting {:?} to Serialized", bytes)))?;
                let measurement = Measurement::from_bytes(bytes);

                if Some(measurement) != last_meas {
                    tokio::spawn(handle_measurement(measurement, device.address()));
                } else {
                    debug!("Duplicate measurement: {:?}", measurement);
                }
                last_meas = Some(measurement);
                ()
            };

            debug!("Received new Manufacturer data from {}: {:?}, Result: {:?}",
                device.address(), data, result);
        }
    }

    info!("Stopped receiving events from {}. Not sure what this means.", name);

    Ok(())
}

pub async fn handle_measurement(meas: Measurement, addr: Address) {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_|{
        time::OffsetDateTime::now_utc()
    });

    let meas = MeasurementRequest {
        measurement: meas,
        timestamp: now,
        sensor_address: addr.0
    };

    let json = serde_json::to_string_pretty(&meas).unwrap_or("error".to_string());

    info!("Cool new measurement: {}", json);
}