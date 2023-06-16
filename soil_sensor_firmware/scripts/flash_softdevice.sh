#!/bin/bash

probe-rs-cli erase --chip nrf52810
probe-rs-cli download --chip nrf52810 --format hex s112_nrf52_7.3.0_softdevice.hex