#![no_std]
#![no_main]

#[macro_use]
extern crate itsybitsy_m0 as hal;
extern crate cortex_m_semihosting;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::{entry, CorePeripherals, Peripherals};
#[cfg(not(feature = "use_semihosting"))]
extern crate panic_abort;
#[cfg(feature = "use_semihosting")]
extern crate panic_semihosting;

static mut I2CS_DEV: Option<hal::sercom::I2CSlave3> = None;

// IMPORTANT!
// For this example to work, you have to setup .gdbinit as follows
// 
// ```
// monitor halt
// load
// monitor reset
// ```
//
// This loads the code, then resets so we get our interrupts.

#[cfg(feature = "use_semihosting")]
macro_rules! dbgprint {
    ($($arg:tt)*) => {
        {
            use cortex_m_semihosting::hio;
            use core::fmt::Write;
            let mut stdout = hio::hstdout().unwrap();
            writeln!(stdout, $($arg)*).ok();
        }
    };
}

#[cfg(not(feature = "use_semihosting"))]
macro_rules! dbgprint {
    ($($arg:tt)*) => {{}};
}


#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    core.NVIC.enable(hal::Interrupt::SERCOM3);
    unsafe {
        core.NVIC.set_priority(hal::Interrupt::SERCOM3, 0);
    }

    let mut pins = hal::Pins::new(peripherals.PORT);

    unsafe {
        I2CS_DEV = Some(hal::i2c_slave(
            &mut clocks,
            peripherals.SERCOM3,
            &mut peripherals.PM,
            pins.sda,
            pins.scl,
            &mut pins.port,
            0x21,
        ));
    }

    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);
    let mut delay = Delay::new(core.SYST, &mut clocks);
    loop {
        delay.delay_ms(200u8);
        red_led.set_high();
        delay.delay_ms(200u8);
        red_led.set_low();
    }
}

interrupt!(SERCOM3, sercom3);

fn sercom3() {
    unsafe { I2CS_DEV.as_mut() }.map(|a| {
        if a.is_read_request() {
            a.write(&[0x1c, 0x9a]);
        }
        a.service_interrupt();
    });
}
