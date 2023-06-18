use btleplug::api::{Central, CentralEvent, CharPropFlags, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::stream::StreamExt;
use std::time::Duration;
use std::fmt;
use btleplug::api::bleuuid::BleUuid;
use tokio::time;
use tokio::sync::broadcast;
use uuid::{Uuid, uuid};
use thiserror::Error;
use log::{debug, info, warn, error};

use crate::sensor_manager;

#[derive(Error, Debug)]
pub enum ListenError {
    #[error("BLEError({:?})", .0)]
    BLEError(#[from] btleplug::Error)
}

pub async fn main_loop() {
    let manager = Manager::new().await.unwrap();

    let adapters = manager.adapters().await.unwrap();

    let join_handles: Vec<tokio::task::JoinHandle<Result<(), ListenError>>> = adapters.into_iter().map(|a: Adapter|{
        tokio::spawn(central_listener(a))
    }).collect();

    for join in join_handles {
        join.await;
    }

    ()
}

async fn central_listener(central: Adapter) -> Result<(), ListenError> {
    debug!("Spawned central_listener task for {}", central.adapter_info().await.unwrap().as_str());
    // Each adapter has an event stream, we fetch via events(),
    // simplifying the type, this will return what is essentially a
    // Future<Result<Stream<Item=CentralEvent>>>.
    let mut events = central.events().await?;

    // start scanning for devices
    // central.start_scan(ScanFilter {
    //     services: vec![SENSOR_SERVICE_UUID]
    // }).await?;
    central.start_scan(ScanFilter::default());

    let (mut tx, _) = broadcast::channel::<CentralEvent>(50);

    tokio::spawn(event_logger(tx.subscribe()));

    // Print based on whatever the event receiver outputs. Note that the event
    // receiver blocks, so in a real program, this should be run in its own
    // thread (not task, as this library does not yet use async channels).
    while let Some(event) = events.next().await {
        tx.send(event.clone());
        if let CentralEvent::DeviceDiscovered(id) = &event {
            let perip = central.peripheral(&id).await.unwrap();
            tokio::spawn(sensor_manager::manage_sensor_wrapper(perip, tx.subscribe()));
        }

    }

    Ok(())
}

async fn event_logger(mut receiver: broadcast::Receiver<CentralEvent>) {
    while let Ok(event) = receiver.recv().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                debug!("DeviceDiscovered: {:?}", id);
            }
            CentralEvent::DeviceConnected(id) => {
                debug!("DeviceConnected: {:?}", id);
            }
            CentralEvent::DeviceDisconnected(id) => {
                debug!("DeviceDisconnected: {:?}", id);
            }
            CentralEvent::ManufacturerDataAdvertisement {
                id,
                manufacturer_data,
            } => {
                debug!(
                    "ManufacturerDataAdvertisement: {:?}, {:?}",
                    id, manufacturer_data
                );
            }
            CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                debug!("ServiceDataAdvertisement: {:?}, {:?}", id, service_data);
            }
            CentralEvent::ServicesAdvertisement { id, services } => {
                let services: Vec<String> =
                    services.into_iter().map(|s| s.to_short_string()).collect();
                debug!("ServicesAdvertisement: {:?}, {:?}", id, services);
            }
            CentralEvent::DeviceUpdated(id) => {
                debug!("DeviceUpdated: {:?}", id);
            }
        }
    }
}