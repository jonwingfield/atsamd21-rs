#![feature(used)]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_semihosting;
extern crate metro_m0 as hal;
#[cfg(not(feature = "use_semihosting"))]
extern crate panic_abort;
extern crate nrf24l01;
#[cfg(feature = "use_semihosting")]
extern crate panic_semihosting;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::{CorePeripherals, Peripherals};
// use embedded_nrf24l01::{Configuration,NRF24L01,CrcMode};
use nrf24l01::NRF24L01;
use cortex_m::asm;

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

fn main() {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::new(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );

    
    let mut pins = hal::pins(peripherals.PORT);


    let spi_master = hal::spi_master(
        &mut clocks,
        1.mhz(),
        peripherals.SERCOM4,
        &mut peripherals.PM,
        pins.sck,
        pins.mosi,
        pins.miso,
        &mut pins.port);

    // asm::bkpt();

    let ce = pins.d0.into_push_pull_output(&mut pins.port);
    let mut csn = pins.d1.into_push_pull_output(&mut pins.port);
    csn.set_high();

    // let mut nrf24 = NRF24L01::new(
    //     ce,
    //     csn,
    //     spi_master).unwrap(); // TODO: panic here or fail gracefully

    // let x: Option<CrcMode> = None;
    // // nrf24.set_rf(&embedded_nrf24l01::DataRate::R250Kbps, 0b010).unwrap();
    // nrf24.set_channel(6).unwrap();
    // nrf24.set_rx_addr(0, b"444").unwrap();
    // nrf24.set_crc(&CrcMode::TwoBytes);
    // nrf24.set_pipes_rx_enable(&[true, true, true, true, true, true]);
    
    // dbgprint!("Channel {}\n", nrf24.get_channel().unwrap());
    // dbgprint!("AutoAck {:?}\n", nrf24.get_auto_ack().unwrap()[0]);
    // dbgprint!("Register {:?}\n", nrf24.get_address_width().unwrap());

    // let mut rx = nrf24.rx().unwrap();
    //
     
    let mut nrf24l01 = NRF24L01::new(spi_master,
                                     csn,
                                     ce,
                                     76,
                                     8).unwrap();

    nrf24l01.set_raddr("serv1".as_bytes()).unwrap();
    nrf24l01.config().unwrap();


    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);
    let mut delay = Delay::new(core.SYST, &mut clocks);
    dbgprint!("Channel {}\n", nrf24l01.get_channel().unwrap());
    // asm::bkpt();
    let a = nrf24l01.get_addr().unwrap();
    red_led.set_low();
    dbgprint!("raddr {} \n", a);

    loop {
        // if let Some(_) = rx.can_read().unwrap() {
        //     let payload = rx.read().unwrap();
        //     let data = payload[0];
        //     if data > 0 {
        //         red_led.set_high();
        //     } else {
        //         red_led.set_low();
        //     }
        // } else {
        //     dbgprint!("Nothing.");
        // }
        //
        //
        if !nrf24l01.is_sending().unwrap() {
            nrf24l01.set_taddr("clie1".as_bytes()).unwrap();
            let mut buffer = [1; 8];
            nrf24l01.send(&buffer).unwrap();
        }
        delay.delay_ms(10u8);
    }
}
