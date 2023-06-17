use std::array::TryFromSliceError;
use btleplug::api::{Central, CentralEvent, Characteristic, CharPropFlags, Manager as _, Peripheral as _, ScanFilter, ValueNotification};
use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use futures::stream::StreamExt;
use std::time::Duration;
use std::fmt;
use btleplug::api::bleuuid::BleUuid;
use tokio::time;
use uuid::{Uuid, uuid};
use log::{debug, info, warn, error};
use soil_sensor_common::{Measurement, Serialized};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use crate::sensor_manager::SensorError::SensorDisconnected;

/// UUID of the Service for sensor measurements.
pub const SENSOR_SERVICE_UUID: Uuid = Uuid::from_u128(0x866a5627_a761_47cc_9976_7457450e8257);
/// UUID of the Service for which we should subscribe to notifications.
pub const SENSOR_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(0x866a5627_a761_47cc_9976_7457450e8258);
/// Name of the sensor starts with this
pub const SENSOR_NAME_PREFIX: &'static str = "BLE Soil Sensor";

#[derive(thiserror::Error, Debug)]
pub enum SensorError {
    #[error("btleplug::Error {:?}", .0)]
    BTLEPlugError(#[from] btleplug::Error),
    #[error("NoOptions")]
    NoOptions,
    #[error("NotASoilSensor (name={})", .0)]
    InvalidSensor(String),
    #[error("SensorDisconnected")]
    SensorDisconnected,
    #[error("ChannelDisconnected: {:#?}", .0)]
    ChannelDisconnected(#[from] RecvError),
}

// TODO: Figure out error handling (And remove this!)
pub async fn manage_sensor_wrapper(sensor: Peripheral, mut receiver: broadcast::Receiver<CentralEvent>) {
    let r = manage_sensor(sensor, receiver).await;
    error!("{:#?}", r);
}

pub async fn manage_sensor(sensor: Peripheral, mut receiver: broadcast::Receiver<CentralEvent>) -> Result<(), SensorError> {
    debug!("Peripheral ID: {}, Connected: {}", sensor.id(), sensor.is_connected().await?);
    let prop = sensor.properties().await?.ok_or(SensorError::NoOptions)?;
    debug!("Properties: {:#?}", prop);

    let name = prop.local_name.unwrap_or("".to_string());

    let has_service = prop.services.contains(&SENSOR_SERVICE_UUID);
    let good_name = name.starts_with(SENSOR_NAME_PREFIX);

    debug!("{}: has_service: {}, good_name: {}", name, has_service, good_name);

    if !has_service || !good_name {
        return Err(SensorError::InvalidSensor(name));
    }


    while !sensor.is_connected().await? {
        debug!("Connecting to {}...", name);
        let r = sensor.connect().await;
        debug!("{:#?}", r);
    }
    debug!("Connected to {}!", name);
    sensor.discover_services().await?;

    let services = sensor.services();
    debug!("{:#?}", services);

    let characteristic: &Characteristic = services.iter()
        // Filter down to services which are the correct UUID.
        // Map to their first characteristic with the correct UUID.
        .filter_map(|s|{
            if s.uuid == SENSOR_SERVICE_UUID {
                s.characteristics.iter()
                    // Filter characteristics with the right UUID, and which can Notify
                    .filter(|c| {
                        c.uuid == SENSOR_CHARACTERISTIC_UUID && c.properties.contains(CharPropFlags::NOTIFY)
                    })
                    // .next() will be None if no appropriate characteristics found
                    .next()
            } else {
                None
            }
        })
        // .next() will be None if no appropriate characteristic found
        .next().ok_or(SensorError::InvalidSensor(name.clone()))?;

    debug!("Subscribing to characteristic {:#?}", characteristic);
    sensor.subscribe(&characteristic).await?;
    debug!("Notification stream starting");
    let mut notification_stream = sensor.notifications().await?;
    // Process while the BLE connection is not broken or stopped.
    loop {
        // error!("{:#?}", receiver.recv().await);
        tokio::select! {
            data_result = notification_stream.next() => {
                let data: ValueNotification = data_result.ok_or(SensorDisconnected)?;
                debug!(
                    "Received data from {:?} [{:?}]: {:?}",
                    name, data.uuid, data.value
                );
                if data.uuid == SENSOR_CHARACTERISTIC_UUID {
                    tokio::spawn(handle_measurement(name.clone(), data.value.clone()));
                }
            },
            event_result = receiver.recv() => {
                if let Ok(CentralEvent::DeviceDisconnected(id)) = event_result {
                    if id == sensor.id() {
                        error!("It disconnected!!!");
                        sensor.disconnect().await?;
                        sensor.connect().await?;
                    }
                }
            }
        }
    }
    Err(SensorDisconnected)
}

#[derive(thiserror::Error, Debug)]
pub enum MeasurementError {
    #[error("Data wrong length ({})", .0)]
    DataLength(#[from] TryFromSliceError),
}

pub async fn handle_measurement(name: String, data: Vec<u8>) -> Result<(), MeasurementError> {
    let data_array = Serialized::try_from(data.as_slice())?;
    let meas = Measurement::from_bytes(data_array);
    info!("{:#?}", meas);

    Ok(())
}