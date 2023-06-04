# TODO: Write a good README

# Building

### Prerequisites:

```shell
cargo install probe-run
cargo install flip-link
cargo install cargo-binutils
cargo install probe-rs-cli
```

### Build:

```shell
cargo build
```

This will use the default target of `thumbv7em-none-eabi` configured in `.cargo/config.toml`.

### Run:

```shell
DEFMT_LOG=trace cargo run --bin soil_sensor_firmware
```

This will use `probe-run`, configured in `.cargo/config.toml`, to run the code on a JLink debug probe.

The environment variable `DEFMT_LOG` controls the log level which will be displayed. It defaults to `error`, which
will only display error messages.