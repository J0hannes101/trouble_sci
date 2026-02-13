use trouble_host::prelude::*;

#[gatt_server]
pub struct CounterServer {
    pub counter_service: CounterService,
}

#[gatt_service(uuid = "0000ffe0-0000-1000-8000-00805f9b34fb")]
pub struct CounterService {
    #[characteristic(uuid = "0000ffe1-0000-1000-8000-00805f9b34fb", read, notify)]
    pub counter: u32,
    #[characteristic(uuid = "0000ffe2-0000-1000-8000-00805f9b34fb", write)]
    pub command: u8,
}
