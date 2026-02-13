use embassy_nrf::{Peri, bind_interrupts, peripherals::*, rng};
use macros::take_resources;
use nrf_sdc::mpsl::{self, MultiprotocolServiceLayer};
use static_cell::StaticCell;
use trouble_host::prelude::*;

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<RNG>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

const L2CAP_TXQ: u8 = 3;
const L2CAP_RXQ: u8 = 3;

fn build_sdc<'d, const N: usize>(
    p: nrf_sdc::Peripherals<'d>,
    rng: &'d mut rng::Rng<embassy_nrf::mode::Async>,
    mpsl: &'d MultiprotocolServiceLayer,
    mem: &'d mut nrf_sdc::Mem<N>,
) -> Result<nrf_sdc::SoftdeviceController<'d>, nrf_sdc::Error> {
    let mut builder = nrf_sdc::Builder::new()?;

    builder = builder.support_extended_feature_set().support_le_2m_phy();

    // Role-specific support
    #[cfg(feature = "peripheral")]
    {
        builder = builder
            .support_peripheral()
            .support_phy_update_peripheral()
            .support_adv()
            .support_connection_subrating_peripheral()
            .support_frame_space_update_peripheral()
            .support_shorter_connection_intervals_peripheral();
    }

    #[cfg(feature = "central")]
    {
        builder = builder
            .support_central()
            .support_phy_update_central()
            .support_connection_subrating_central()
            .support_frame_space_update_central()
            .support_shorter_connection_intervals_central();
    }

    builder = builder.support_lowest_frame_space().buffer_cfg(
        DefaultPacketPool::MTU as u16,
        DefaultPacketPool::MTU as u16,
        L2CAP_TXQ,
        L2CAP_RXQ,
    )?;

    let sdc = builder.build(p, rng, mpsl, mem);

    sdc
}

#[take_resources]
pub struct BleResources<'p> {
    pub rtc0: Peri<'p, RTC0>,
    pub timer0: Peri<'p, TIMER0>,
    pub temp: Peri<'p, TEMP>,
    pub rng: Peri<'p, RNG>,
    pub ppi_ch17: Peri<'p, PPI_CH17>,
    pub ppi_ch18: Peri<'p, PPI_CH18>,
    pub ppi_ch19: Peri<'p, PPI_CH19>,
    pub ppi_ch20: Peri<'p, PPI_CH20>,
    pub ppi_ch21: Peri<'p, PPI_CH21>,
    pub ppi_ch22: Peri<'p, PPI_CH22>,
    pub ppi_ch23: Peri<'p, PPI_CH23>,
    pub ppi_ch24: Peri<'p, PPI_CH24>,
    pub ppi_ch25: Peri<'p, PPI_CH25>,
    pub ppi_ch26: Peri<'p, PPI_CH26>,
    pub ppi_ch27: Peri<'p, PPI_CH27>,
    pub ppi_ch28: Peri<'p, PPI_CH28>,
    pub ppi_ch29: Peri<'p, PPI_CH29>,
    pub ppi_ch30: Peri<'p, PPI_CH30>,
    pub ppi_ch31: Peri<'p, PPI_CH31>,
}

pub fn init_ble<'d>(
    p: BleResources<'static>,
    spawner: embassy_executor::Spawner,
) -> nrf_sdc::SoftdeviceController<'d> {
    let mpsl_p =
        mpsl::Peripherals::new(p.rtc0, p.timer0, p.temp, p.ppi_ch19, p.ppi_ch30, p.ppi_ch31);

    let lfclk_cfg = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl::raw::MPSL_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_CTIV as u8,
        rc_temp_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_TEMP_CTIV as u8,
        accuracy_ppm: mpsl::raw::MPSL_DEFAULT_CLOCK_ACCURACY_PPM as u16,
        skip_wait_lfclk_started: mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };

    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    static RNG: StaticCell<embassy_nrf::rng::Rng<'static, embassy_nrf::mode::Async>> =
        StaticCell::new();
    static SDC_MEM: StaticCell<nrf_sdc::Mem<7_500>> = StaticCell::new();

    let mpsl = MPSL.init(mpsl::MultiprotocolServiceLayer::new(mpsl_p, Irqs, lfclk_cfg).unwrap());
    spawner.spawn(mpsl_task(&*mpsl)).unwrap();

    let sdc_p = nrf_sdc::Peripherals::new(
        p.ppi_ch17, p.ppi_ch18, p.ppi_ch20, p.ppi_ch21, p.ppi_ch22, p.ppi_ch23, p.ppi_ch24,
        p.ppi_ch25, p.ppi_ch26, p.ppi_ch27, p.ppi_ch28, p.ppi_ch29,
    );

    let rng = RNG.init(rng::Rng::new(p.rng, Irqs));
    let sdc_mem = SDC_MEM.init(nrf_sdc::Mem::<7_500>::new());

    build_sdc(sdc_p, rng, mpsl, sdc_mem).unwrap()
}
