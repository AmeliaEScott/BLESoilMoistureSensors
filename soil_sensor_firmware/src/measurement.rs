use defmt::Format;

#[derive(Debug, Format)]
pub struct Measurement {
    pub id: u16,
    pub moisture_frequency: u32,
    pub temperature: i32,
    pub capacitor_voltage: i16
}

pub type Serialized = [u8; 12];

impl Measurement {
    pub fn to_bytes(&self) -> Serialized {
        let mut bytes: Serialized = [0u8; 12];
        bytes[0..2].copy_from_slice(self.id.to_be_bytes().as_slice());
        bytes[2..6].copy_from_slice(self.moisture_frequency.to_be_bytes().as_slice());
        bytes[6..10].copy_from_slice(self.temperature.to_be_bytes().as_slice());
        bytes[10..12].copy_from_slice(self.capacitor_voltage.to_be_bytes().as_slice());

        bytes
    }

    pub fn from_bytes(bytes: Serialized) -> Self {
        let id = u16::from_be_bytes(<[u8; 2]>::try_from(&bytes[0..2]).unwrap());
        let moisture_frequency = u32::from_be_bytes(<[u8; 4]>::try_from(&bytes[2..6]).unwrap());
        let temperature = i32::from_be_bytes(<[u8; 4]>::try_from(&bytes[6..10]).unwrap());
        let capacitor_voltage = i16::from_be_bytes(<[u8; 2]>::try_from(&bytes[10..12]).unwrap());

        Self {
            id, moisture_frequency, temperature, capacitor_voltage
        }
    }
}