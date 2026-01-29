#![allow(unused)]
use bt_hci::{
    cmd::{
        info::ReadLocalSupportedCmds,
        le::{
            LeConnectionRateRequest, LeFrameSpaceUpdate, LeReadLocalSupportedFeatures,
            LeReadMinimumSupportedConnectionInterval, LeSetDefaultRateParameters, LeSetPhy,
        },
    },
    controller::{ControllerCmdAsync, ControllerCmdSync},
};
use embassy_futures::{
    join::join,
    select::{Either, select},
};
use embassy_time::{Duration, Instant, Timer};
use log::{info, warn};
use static_cell::StaticCell;
use trouble_host::prelude::*;
use trouble_host::gatt::GattConnectionEvent;

#[cfg(feature = "peripheral")]
use crate::gatt::CounterServer;

const ADVERTISE_NAME: &str = "BLE-SCI-TEST";

const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 3;

const SERVICE_UUID_BYTES: [u8; 16] = [
    0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80, // 8000
    0x00, 0x10, // 1000
    0x00, 0x00, // 0000
    0xe0, 0xff, 0x00, 0x00, // ffe0
];
const SERVICE_UUID: Uuid = Uuid::Uuid128(SERVICE_UUID_BYTES);

const CHAR_UUID: Uuid = Uuid::Uuid128([
    0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80, 0x00, 0x10, 0x00, 0x00, 0xe1, 0xff, 0x00, 0x00,
]);

const CHAR_CMD_UUID: Uuid = Uuid::Uuid128([
    0xfb, 0x34, 0x9b, 0x5f, 0x80, 0x00, 0x00, 0x80, 0x00, 0x10, 0x00, 0x00, 0xe2, 0xff, 0x00, 0x00,
]);

const PERIPHERAL_ADDR_BYTES: [u8; 6] = [0xff, 0x1f, 0x1f, 0x1f, 0x1f, 0xc0];
const CENTRAL_ADDR_BYTES: [u8; 6] = [0xaa, 0x2f, 0x2f, 0x2f, 0x2f, 0xc0];

static RESOURCES: StaticCell<
    HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>,
> = StaticCell::new();

#[cfg(feature = "peripheral")]
static SERVER: StaticCell<CounterServer<'static>> = StaticCell::new();

#[cfg(all(feature = "peripheral", feature = "central"))]
compile_error!("enable only one of the features: `peripheral` or `central`");

pub async fn run<C>(controller: C)
where
    C: Controller
        + ControllerCmdSync<LeReadLocalSupportedFeatures>
        + ControllerCmdSync<LeReadMinimumSupportedConnectionInterval>
        + ControllerCmdSync<LeConnectionRateRequest>
        + ControllerCmdSync<ReadLocalSupportedCmds>
        + ControllerCmdAsync<LeSetPhy>
        + ControllerCmdSync<LeFrameSpaceUpdate>
        + ControllerCmdSync<LeSetDefaultRateParameters>,
{
    #[cfg(feature = "peripheral")]
    let address = Address::random(PERIPHERAL_ADDR_BYTES);

    #[cfg(feature = "central")]
    let address = Address::random(CENTRAL_ADDR_BYTES);

    info!("Starting BLE Stack with address {:?}", address);

    let resources = RESOURCES.init(HostResources::new());
    let stack = trouble_host::new(controller, resources).set_random_address(address);

    #[cfg(feature = "peripheral")]
    {
        let Host {
            mut peripheral,
            mut runner,
            ..
        } = stack.build();

        let server = SERVER.init(
            CounterServer::new_with_config(GapConfig::Peripheral(PeripheralConfig {
                name: ADVERTISE_NAME,
                appearance: &appearance::power_device::GENERIC_POWER_DEVICE,
            }))
            .unwrap(),
        );

        join(runner.run(), async {
            loop {
                info!("Advertising...");

                let mut adv_data = [0; 31];
                let mut scan_data = [0; 31];

                let len_adv = AdStructure::encode_slice(
                    &[
                        AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                        AdStructure::ServiceUuids128(&[SERVICE_UUID_BYTES]),
                    ],
                    &mut adv_data,
                )
                .unwrap();

                let len_scan = AdStructure::encode_slice(
                    &[AdStructure::CompleteLocalName(ADVERTISE_NAME.as_bytes())],
                    &mut scan_data,
                )
                .unwrap();

                let advertiser = peripheral
                    .advertise(
                        &Default::default(),
                        Advertisement::ConnectableScannableUndirected {
                            adv_data: &adv_data[..len_adv],
                            scan_data: &scan_data[..len_scan],
                        },
                    )
                    .await
                    .unwrap();

                let connection = advertiser.accept().await.unwrap();
                info!("Central connected!");

                let mut counter: u32 = 0;
                let gatt_conn = connection.with_attribute_server(server).unwrap();
                let mut last_tick: Option<Instant> = None;

                loop {
                    let event = gatt_conn.next().await;

                    match event {
                        GattConnectionEvent::ConnectionRateChanged { conn_interval, .. } => {
                            log::info!("connection changed - rate: {}us", conn_interval.as_micros())
                        }
                        GattConnectionEvent::Disconnected { reason } => {
                            info!("Disconnected (reason: {:?})", reason);
                            break;
                        }
                        GattConnectionEvent::Gatt { event } => {
                            match event {
                                GattEvent::Write { .. } => {
                                    let now = Instant::now();

                                    if let Some(prev) = last_tick {
                                        let elapsed = now - prev;
                                        if counter % 100 == 0 {
                                            info!(
                                                "Count: {} | Interval: {}ms",
                                                counter,
                                                elapsed.as_millis()
                                            );
                                        }
                                    }
                                    last_tick = Some(now);

                                    // Update DB and notify
                                    server
                                        .counter_service
                                        .counter
                                        .set(&server, &counter)
                                        .unwrap();
                                    let _ = server
                                        .counter_service
                                        .counter
                                        .notify(&gatt_conn, &counter)
                                        .await;
                                    counter = counter.wrapping_add(1);
                                }
                                _ => {}
                            }
                        }
                        GattConnectionEvent::ConnectionParamsUpdated { conn_interval, .. } => {
                            info!("Average Interval: {}ms", conn_interval.as_millis());
                        }

                        GattConnectionEvent::ConnectionRateChanged { conn_interval, .. } => {
                            log::info!(
                                "sci: connection_rate_changed: {}us",
                                conn_interval.as_micros()
                            );
                        }
                        _ => {}
                    }
                }
            }
        })
        .await;
    }

    #[cfg(feature = "central")]
    {
        let Host {
            mut central,
            mut runner,
            ..
        } = stack.build();
        let target = Address::random(PERIPHERAL_ADDR_BYTES);

        let config = ConnectConfig {
            connect_params: Default::default(),
            scan_config: ScanConfig {
                filter_accept_list: &[(target.kind, &target.addr)],
                ..Default::default()
            },
        };

        join(runner.run(), async {
            loop {
                info!("Connecting to {:?}...", target);
                match central.connect(&config).await {
                    Ok(conn) => {
                        use bt_hci::{AsHciBytes, param::SpacingTypes};

                        match stack.command(LeReadLocalSupportedFeatures::new()).await {
                            Ok(supported) => {
                                info!("supported features: {:?}", supported.as_hci_bytes())
                            }
                            Err(e) => warn!("Failed to read supported features: {:?}", e),
                        }

                        let connection_params = ConnectParams {
                            min_connection_interval: Duration::from_micros(7500),
                            max_connection_interval: Duration::from_micros(7500),
                            max_latency: 0,
                            min_event_length: Duration::from_micros(0),
                            max_event_length: Duration::from_micros(0),
                            supervision_timeout: Duration::from_millis(500),
                        };

                        match conn.set_phy(&stack, PhyKind::Le2M).await {
                            Ok(_) => info!("PHY set to LE 2M"),
                            Err(e) => warn!("Failed to set PHY: {:?}", e),
                        }

                        match conn.update_connection_params(&stack, &connection_params).await {
                            Ok(_) => info!("Connection parameters updated to 7.5ms"),
                            Err(e) => warn!("Failed to update connection parameters: {:?}", e),
                        }

                        match conn
                            .update_frame_space(
                                &stack,
                                Duration::from_micros(0),
                                Duration::from_micros(1000),
                                PhyMask::new().set_le_2m_phy(true),
                                SpacingTypes::new()
                                    .set_t_ifs_acl_cp(true)
                                    .set_t_ifs_acl_pc(true)
                                    .set_t_mces(true),
                            )
                            .await
                        {
                            Ok(_) => info!("Frame space updated"),
                            Err(e) => warn!("Failed to update frame space: {:?}", e),
                        }

                        match stack.command(ReadLocalSupportedCmds::new()).await {
                            Ok(res) => info!("LE command mask: {:?}", res.as_hci_bytes()[48]),
                            Err(e) => warn!("Failed to read local supported commands: {:?}", e),
                        }

                        match stack.read_minimum_supported_connection_interval().await {
                            Ok(res) => info!(
                                "Minimum supported connection interval: {:?}us",
                                res.minimum_supported_connection_interval.as_micros()
                            ),
                            Err(e) => warn!(
                                "Failed to read minimum supported connection interval: {:?}",
                                e
                            ),
                        }

                        let conn_rate_params = ConnectRateParams {
                            min_connection_interval: Duration::from_micros(750),
                            max_connection_interval: Duration::from_micros(4000),
                            subrate_min: 1,
                            subrate_max: 1,
                            max_latency: 0,
                            continuation_number: 0,
                            supervision_timeout: Duration::from_secs(2),
                            min_ce_length: Duration::from_micros(750),
                            max_ce_length: Duration::from_micros(4000),
                        };

                        match central
                            .set_default_connection_rate_parameters(&conn_rate_params)
                            .await
                        {
                            Ok(_) => info!("Default rate parameters set"),
                            Err(e) => warn!("Failed to set default rate parameters: {:?}", e),
                        }

                        match conn
                            .request_connection_rate(&stack, &conn_rate_params)
                            .await
                        {
                            Ok(_) => info!("Connection rate request sent"),
                            Err(e) => warn!("Connection rate request failed: {:?}", e),
                        }

                        let client = match GattClient::<_, DefaultPacketPool, 10>::new(
                            &stack, &conn,
                        )
                        .await
                        {
                            Ok(c) => c,
                            Err(e) => {
                                warn!("Failed to create GATT client: {:?}", e);
                                continue;
                            }
                        };

                        let _ = join(client.task(), async {
                            let (counter_char, command_char) = loop {
                                if let Ok(services) = client.services_by_uuid(&SERVICE_UUID).await {
                                    if let Some(service) = services.first() {
                                        let c = client
                                            .characteristic_by_uuid::<u32>(service, &CHAR_UUID)
                                            .await;
                                        let cmd = client
                                            .characteristic_by_uuid::<u8>(service, &CHAR_CMD_UUID)
                                            .await;
                                        if let (Ok(c), Ok(cmd)) = (c, cmd) {
                                            break (c, cmd);
                                        }
                                    }
                                }
                                Timer::after(Duration::from_millis(500)).await;
                            };

                            let mut listener = match client.subscribe(&counter_char, false).await {
                                Ok(l) => l,
                                Err(e) => {
                                    warn!("Failed to subscribe: {:?}", e);
                                    return;
                                }
                            };

                            info!("Subscribed. Starting Ping-Pong.");
                            if let Err(e) = client.write_characteristic(&command_char, &[1u8]).await
                            {
                                warn!("Failed to send initial ping: {:?}", e);
                            }

                            loop {
                                let _ = listener.next().await;
                                if let Err(e) =
                                    client.write_characteristic(&command_char, &[1u8]).await
                                {
                                    warn!("Ping-pong broken: {:?}", e);
                                    break;
                                }
                            }
                        })
                        .await;
                    }
                    Err(e) => warn!("Connect failed: {:?}", e),
                }
                Timer::after(Duration::from_secs(2)).await;
            }
        })
        .await;
    }
}
