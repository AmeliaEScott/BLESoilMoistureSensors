use std::env;

fn main() {
    // Set default value for SENSOR_ID for convenience in IDE
    if let Err(_) = env::var("SENSOR_ID") {
        println!("cargo:rustc-env=SENSOR_ID=0000");
    }
}