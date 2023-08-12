// @generated automatically by Diesel CLI.

diesel::table! {
    measurements (id) {
        id -> Int4,
        sensor_id -> Int4,
        sequence -> Int4,
        moisture -> Int4,
        temperature -> Float8,
        capacitor_voltage -> Float8,
        time -> Timestamptz,
    }
}

diesel::table! {
    sensors (id) {
        id -> Int4,
        hardware_address -> Macaddr,
        description -> Nullable<Text>,
    }
}

diesel::joinable!(measurements -> sensors (sensor_id));

diesel::allow_tables_to_appear_in_same_query!(
    measurements,
    sensors,
);
