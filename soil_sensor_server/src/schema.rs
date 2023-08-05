// @generated automatically by Diesel CLI.

diesel::table! {
    sensor_readings (id) {
        id -> Int4,
        sensor_id -> Int4,
        hardware_address -> Macaddr,
        sequence -> Int4,
        moisture -> Int4,
        temperature -> Float8,
        capacitor_voltage -> Float8,
        time -> Timestamptz,
    }
}
