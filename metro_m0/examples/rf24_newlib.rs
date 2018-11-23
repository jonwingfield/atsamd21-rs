#![feature(used)]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_semihosting;
extern crate metro_m0 as hal;
#[cfg(not(feature = "use_semihosting"))]
extern crate panic_abort;
extern crate embedded_nrf24l01;
#[cfg(feature = "use_semihosting")]
extern crate panic_semihosting;
extern crate embedded_hal;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::{CorePeripherals, Peripherals};
use embedded_nrf24l01::{Configuration,NRF24L01,CrcMode,PAControl};
use cortex_m::asm;
use embedded_hal::digital::OutputPin;
use embedded_hal::blocking::spi::Transfer as SPITransfer;

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

#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
enum GarageDoorMessage {
    SetTargetState = 0b01,
    StateChanged = 0b10,
    GetTargetState = 0b11,
}

#[derive(PartialEq, Clone, Copy)]
#[repr(u8)]
enum GarageDoorTargetState {
    Closed = 0b00,
    Open = 0b01,
}

#[derive(PartialEq, Clone, Copy, Debug)]
#[repr(u8)]
enum GarageDoorPosition {
    Closed = 0,
    Closing = 1,
    Opening = 2,
    Open = 3,
    Stopped = 4,
}

struct GarageDoorController<CE: OutputPin, CSN: OutputPin, SPI: SPITransfer<u8>>  {
    nrf24: NRF24L01<CE, CSN, SPI>
}
 
use GarageDoorPosition::*;

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

    let mut nrf24 = NRF24L01::new(
        ce,
        csn,
        spi_master).unwrap(); // TODO: panic here or fail gracefully

    nrf24.set_rf(embedded_nrf24l01::DataRate::R250Kbps, PAControl::PAMin).unwrap();
    nrf24.set_frequency(76).unwrap();
    nrf24.set_tx_addr(b"serv1").unwrap();
    nrf24.set_crc(Some(CrcMode::TwoBytes)).unwrap();
    nrf24.set_pipes_rx_lengths(&[Some(2),Some(2),Some(2),Some(2),Some(2),Some(2)]).unwrap();
    nrf24.set_pipes_rx_enable(&[true, true, true, true, true, true]).unwrap();
    nrf24.set_auto_ack(&[true, true, true, true, true, true]).unwrap();
    nrf24.set_auto_retransmit(15, 15).unwrap();
    nrf24.flush_tx().unwrap();
    nrf24.flush_rx().unwrap();

    dbgprint!("Channel {}\n", nrf24.get_frequency().unwrap());
    dbgprint!("AutoAck {:?}\n", nrf24.get_auto_ack().unwrap()[1]);
    dbgprint!("Register {:?}\n", nrf24.get_address_width().unwrap());

    let mut rx = nrf24.rx().unwrap();
    
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    let mut gd_open_switch = pins.d10.into_open_drain_output(&mut pins.port);
    let gd_position_top = pins.d11.into_pull_up_input(&mut pins.port);
    let gd_position_bottom = pins.d12.into_pull_up_input(&mut pins.port);

    red_led.set_high();
    let mut delay_clock = Delay::new(core.SYST, &mut clocks);
    
    dbgprint!("newlib1!\n");

    let mut delay: (u8,u8);
    let mut position = if gd_position_top.is_low() { GarageDoorPosition::Open 
        } else if gd_position_bottom.is_low() { GarageDoorPosition::Closed 
        } else { GarageDoorPosition::Stopped };

    loop {
        if let Some(_) = rx.can_read().unwrap() {
            let payload = rx.read().unwrap();
            if payload[0] == GarageDoorMessage::SetTargetState as u8 {
                gd_open_switch.set_high();
                delay_clock.delay_ms(250u8);
                delay_clock.delay_ms(250u8);
                gd_open_switch.set_low();
                position = match position {
                    Open => Closing,
                    Closed => Opening,
                    Opening => Stopped,
                    Closing => Stopped,
                    Stopped => Opening, // TODO: this is a guess. Base it on the prev state if known
                };
                dbgprint!("Position: {:?}\n", position);
            } else if payload[0] == GarageDoorMessage::GetTargetState as u8 {
                let mut tx = rx.standby().tx().unwrap();
                if !tx.send_sync(&[GarageDoorMessage::StateChanged as u8, position as u8]).unwrap() {
                    dbgprint!("No ack received");
                }
                rx = tx.standby().unwrap().rx().unwrap();
                dbgprint!("Sent position update {:?}\n", position);
            }
        } 


        if gd_position_top.is_low() {
            if position != Open {
                position = Open;
                let mut tx = rx.standby().tx().unwrap();
                if !tx.send_sync(&[GarageDoorMessage::StateChanged as u8, position as u8]).unwrap() {
                    dbgprint!("No ack received");
                }
                // asm::bkpt();
                rx = tx.standby().unwrap().rx().unwrap();
                dbgprint!("Position changed: {:?}\n", position);
            }
            delay = (200u8, 0u8);
        } else if gd_position_bottom.is_low() {
            if position != Closed {
                // asm::bkpt();
                position = Closed;
                let mut tx = rx.standby().tx().unwrap();
                tx.send(&[GarageDoorMessage::StateChanged as u8, position as u8]).unwrap();
                rx = tx.standby().unwrap().rx().unwrap();
                dbgprint!("Position changed: {:?}\n", position);
            }
            delay = (0u8, 200u8);
        } else {
            if position == Open {
                // asm::bkpt();
                position = Closing;
                let mut tx = rx.standby().tx().unwrap();
                tx.send(&[GarageDoorMessage::StateChanged as u8, position as u8]).unwrap();
                rx = tx.standby().unwrap().rx().unwrap();
                dbgprint!("Position changed: {:?}\n", position);
            } else if position == Closed {
                // asm::bkpt();
                position = Opening;
                let mut tx = rx.standby().tx().unwrap();
                tx.send(&[GarageDoorMessage::StateChanged as u8, position as u8]).unwrap();
                rx = tx.standby().unwrap().rx().unwrap();
                dbgprint!("Position changed: {:?}\n", position);
            }
            delay = (200u8, 200u8);
        }

        red_led.set_high();
        delay_clock.delay_ms(delay.0);
        red_led.set_low();
        delay_clock.delay_ms(delay.1);
    }
}
