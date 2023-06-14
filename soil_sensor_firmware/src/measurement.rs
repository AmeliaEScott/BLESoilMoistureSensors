use defmt::Format;

#[derive(Debug, Format)]
pub struct Measurement {
    pub moisture_frequency: u32,
    pub temperature: u16,
    pub capacitor_voltage: i16
}