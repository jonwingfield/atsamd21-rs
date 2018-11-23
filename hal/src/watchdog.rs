use super::clock::{ClockGenId, ClockSource, GenericClockController};

use target_device::gclk::clkctrl::GENR::*;
use target_device::gclk::genctrl::SRCR::*;
pub use target_device::wdt::config::PERW as TimeoutPeriod;
pub use target_device::WDT;

pub struct WatchdogTimer {
    wdt: WDT,
}

impl WatchdogTimer {
    pub fn new(
        clocks: &mut GenericClockController,
        wdt: WDT,
        timeout_period: TimeoutPeriod,
    ) -> WatchdogTimer {
        // use the ultra low power internal 32k oscillator as the clock for the wdt
        let gclk2 = clocks
            .configure_gclk_divider_and_source(GCLK3, 32, OSCULP32K, false)
            .unwrap();

        clocks.wdt(&gclk2).unwrap();
        // wait for update to complete.
        // TODO: necessary? The above syncs the GCLK register
        while wdt.status.read().syncbusy().bit_is_set() {}

        // set the timeoutperiod
        wdt.config.modify(|_, w| w.per().variant(timeout_period));

        // enable always-on mode, for simplicity
        wdt.ctrl.modify(|_, w| w.alwayson().set_bit());
        // wait for update to complete
        while wdt.status.read().syncbusy().bit_is_set() {}

        WatchdogTimer { wdt }
    }

    pub fn clear(&mut self) {
        // write the special clear key (0xA5 or 165 in dec) to reset the timer
        self.wdt.clear.write(|w| w.clear().key());
        // wait for update to complete
        while self.wdt.status.read().syncbusy().bit_is_set() {}
    }
}
