use defmt::Format;

#[derive(Debug, Format)]
pub struct Measurement {
    pub moisture_frequency: u32,
    pub temperature: u16,
    pub capacitor_voltage: i16
}

impl Measurement {
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];

        bytes[0..4].copy_from_slice(self.moisture_frequency.to_be_bytes().as_slice());
        bytes[4..6].copy_from_slice(self.temperature.to_be_bytes().as_slice());
        bytes[6..8].copy_from_slice(self.capacitor_voltage.to_be_bytes().as_slice());

        bytes
    }

    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        let moisture_frequency = u32::from_be_bytes(<[u8; 4]>::try_from(&bytes[0..4]).unwrap());
        let temperature = u16::from_be_bytes(<[u8; 2]>::try_from(&bytes[4..6]).unwrap());
        let capacitor_voltage = i16::from_be_bytes(<[u8; 2]>::try_from(&bytes[6..8]).unwrap());

        Self {
            moisture_frequency, temperature, capacitor_voltage
        }
    }
}