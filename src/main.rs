#![no_std]
#![no_main]

use embassy_executor::Spawner;
use log::{LevelFilter, info};
use rtt_target::{rprintln, rtt_init_print};

mod ble;
#[cfg(feature = "peripheral")]
mod gatt;
mod nrf;

use nrf::*;

// --- Panic handler ---
#[panic_handler]
fn panic(e: &core::panic::PanicInfo) -> ! {
    rprintln!("PANIC: {}", e);
    loop {}
}

// --- RTT Logger ---
struct RttLogger;
impl log::Log for RttLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= LevelFilter::Info
    }
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            rprintln!("[{}] {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}
static LOGGER: RttLogger = RttLogger;

fn init_logging() {
    rtt_init_print!();
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Info);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_logging();

    let p = embassy_nrf::init(Default::default());
    info!("Embassy initialized!");

    // init BLE Controller
    let ble_resources = take_ble_resources!(p);
    let sdc = nrf::init_ble(ble_resources, spawner);

    // Run BLE stack
    ble::run(sdc).await;
}
