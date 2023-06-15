#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "defmt")]
use defmt::Format;

#[cfg_attr(feature = "defmt", derive(Format))]
#[derive(Copy, Clone)]
pub struct Measurement {
    pub id: u16,
    pub moisture_frequency: u32,
    pub temperature: i32,
    pub capacitor_voltage: i16,
    pub sequence: u16,
}

pub type Serialized = [u8; 14];

impl Measurement {
    pub fn to_bytes(&self) -> Serialized {
        let mut bytes: Serialized = Serialized::default();
        bytes[0..2].copy_from_slice(self.id.to_be_bytes().as_slice());
        bytes[2..6].copy_from_slice(self.moisture_frequency.to_be_bytes().as_slice());
        bytes[6..10].copy_from_slice(self.temperature.to_be_bytes().as_slice());
        bytes[10..12].copy_from_slice(self.capacitor_voltage.to_be_bytes().as_slice());
        bytes[12..14].copy_from_slice(self.sequence.to_be_bytes().as_slice());

        bytes
    }

    pub fn from_bytes(bytes: Serialized) -> Self {
        let id = u16::from_be_bytes(<[u8; 2]>::try_from(&bytes[0..2]).unwrap());
        let moisture_frequency = u32::from_be_bytes(<[u8; 4]>::try_from(&bytes[2..6]).unwrap());
        let temperature = i32::from_be_bytes(<[u8; 4]>::try_from(&bytes[6..10]).unwrap());
        let capacitor_voltage = i16::from_be_bytes(<[u8; 2]>::try_from(&bytes[10..12]).unwrap());
        let sequence = u16::from_be_bytes(<[u8; 2]>::try_from(&bytes[12..14]).unwrap());

        Self {
            id, moisture_frequency, temperature, capacitor_voltage, sequence
        }
    }
}
