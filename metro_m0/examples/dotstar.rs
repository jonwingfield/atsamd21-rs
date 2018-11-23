#![feature(used)]
#![no_std]

extern crate cortex_m;
#[cfg(feature = "use_semihosting")]
extern crate cortex_m_semihosting;
extern crate metro_m0 as hal;
#[cfg(not(feature = "use_semihosting"))]
extern crate panic_abort;
#[cfg(feature = "use_semihosting")]
extern crate panic_semihosting;
extern crate embedded_hal;
extern crate adafruit_dotstar;
extern crate typenum;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::{CorePeripherals, Peripherals};
use embedded_hal::digital::OutputPin;
use embedded_hal::blocking::spi::Write;
use embedded_hal::blocking::delay::DelayUs;
use typenum::U1;
use adafruit_dotstar::AdafruitDotStar;

struct SoftSpi<SCK: OutputPin, MOSI: OutputPin, D: DelayUs<u32>> {
    sck: SCK,
    mosi: MOSI,
    delay: D,
}

#[derive(Debug)]
pub struct NoSpiError;

impl <SCK: OutputPin, MOSI: OutputPin, D: DelayUs<u32>> SoftSpi<SCK, MOSI, D> {
    pub fn new(mut sck: SCK, mut mosi: MOSI, delay: D) -> Self {
        sck.set_low();
        mosi.set_low();
        SoftSpi {
            sck: sck,
            mosi: mosi,
            delay: delay
        }
    }

    fn write_byte(&mut self, byte: u8) {
        let mut i = 0x80;
        while i > 0 {
            if i & byte > 0 { 
                self.mosi.set_high();
            } else {
                self.mosi.set_low();
            }
            self.sck.set_high();
            self.delay.delay_us(50);
            self.sck.set_low();
            self.delay.delay_us(50);
            i >>= 1;
        };
    }

    fn delay_us(&mut self, d: u32) {
        self.delay.delay_us(d);
    }
}

impl <SCK: OutputPin, MOSI: OutputPin, D: DelayUs<u32>> Write<u8> for SoftSpi<SCK, MOSI, D> {
    type Error = NoSpiError;


    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        for byte in words {
            self.write_byte(*byte);
        }
        Ok(())
    }
}


// fn set_color<SPI: Write<u8>>(spi: &mut SPI, color: u32) -> Result<(), SPI::Error> {
//     spi.write(&[0x0, 0x0, 0x0, 0x0, 0xFF, 
//               color as u8, (color >> 8) as u8, (color >> 16) as u8,
//               0xFF, 0xFF, 0xFF, 0xFF])?;
//     Ok(())
// }

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
    
    let sck = pins.dotstar_sck.into_push_pull_output(&mut pins.port);
    let dotstar_mosi = pins.dotstar_mosi.into_push_pull_output(&mut pins.port);
    
    let spi = SoftSpi::new(sck, dotstar_mosi, 
                               Delay::new(core.SYST, &mut clocks));

    // let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    let mut dotstar = AdafruitDotStar::<_, U1>::new(spi);
    dotstar.set_pixel_color(0, 0x90, 0x90, 0);
    dotstar.show().unwrap();
    // set_color(&mut spi, 0x404000).expect("err");

    let mut rgb = [250u8, 0u8, 0u8];
    let mut index = 1;
    let mut index2 = 0;

    loop {

        rgb[index] += 2;
        rgb[index2] -= 2;
        if rgb[index2] < 2 { 
            index += 1;
            index2 += 1;
            if index > 2 {
                index = 0;
            }
            if index2 > 2 {
                index2 = 0;
            }
        }

        dotstar.set_pixel_color(0, rgb[0], rgb[1], rgb[2]);
        dotstar.show().unwrap();


        // spi.delay_us(200 * 1000);
        // red_led.set_high();
        // spi.delay_us(200 * 1000);
        // red_led.set_low();
    }
}
