#![feature(used)]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_semihosting;
extern crate metro_m0 as hal;
#[cfg(not(feature = "use_semihosting"))]
extern crate panic_abort;
#[cfg(feature = "use_semihosting")]
extern crate panic_semihosting;

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

use cortex_m::asm;
use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::Adc;
use hal::{CorePeripherals, Peripherals};

fn main() {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::new(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    dbgprint!("Initializing ADC!\n");
    // asm::bkpt();
    let mut adc = Adc::new(&mut clocks, &mut peripherals.PM, peripherals.ADC);
    let mut delay = Delay::new(core.SYST, &mut clocks);

    dbgprint!("About to read from ADC!\n");

    loop {
        let value = adc.read_sync();
        dbgprint!("ADC value: {}\n", value);
        delay.delay_ms(200u8);
    }

    // let mut pins = hal::pins(peripherals.PORT);

    // // setup pin as an analog input
    // pins.a0.into_function_b(&mut pins.port);

    // let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);
    // loop {
    //     red_led.set_high();
    //     delay.delay_ms(200u8);
    //     red_led.set_low();
    // }
}

// interrupt!();

// fn
