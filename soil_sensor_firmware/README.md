# TODO: Write a good README

# Building

### Prerequisites:

```shell
sudo apt install -y libudev-dev librust-libudev-dev librust-libudev-sys-dev libusb-1.0-0-dev
cargo install probe-run
cargo install flip-link
cargo install cargo-binutils
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh
rustup target add thumbv7em-none-eabi --toolchain=nightly
```

### Build:

```shell
cargo +nightly build
```

This will use the default target of `thumbv7em-none-eabi` configured in `.cargo/config.toml`.

### Run:

```shell
DEFMT_LOG=trace cargo +nightly run --bin soil_sensor_firmware
```

This will use `probe-run`, configured in `.cargo/config.toml`, to run the code on a JLink debug probe.

The environment variable `DEFMT_LOG` controls the log level which will be displayed. It defaults to `error`, which
will only display error messages.