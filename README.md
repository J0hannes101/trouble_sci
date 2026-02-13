# BLE Subrate Connection Interval (SCI) Test

This project is a test application for demonstrating and testing BLE Subrate Connection Intervals (SCI) on nRF microcontrollers. It utilizes the `embassy` async runtime, `trouble-host` for the BLE host stack, and the `nrf-sdc` for the SoftDevice Controller.

The application can be built for ether central or peripheral, selected by a feature flag.

## Target Hardware

This project is primarily developed for the **nRF52840** microcontroller (tho not fully working as of now). For other Nordic devices, specific features and build configurations may require adjustments.

## Building

To build the project, you must select one of the feature flags.

**Build for Peripheral Role:**
```shell
cargo build --features peripheral
```

**Build for Central Role:**
```shell
cargo build --features central
```
