// Note: section 7.2.3 shows which pins support I2C Hs mode

use sercom::pads::*;
use target_device::sercom0::I2CS;
use target_device::{SERCOM0, SERCOM1, SERCOM2, SERCOM3, PM};
#[cfg(feature = "samd21g18a")]
use target_device::{SERCOM4, SERCOM5};

macro_rules! i2c {
    ([
        $($Type:ident: ($pad0:ident, $pad1:ident, $SERCOM:ident, $powermask:ident, $clock:ident),)+
    ]) => {
        $(
/// Represents the Sercom instance configured to act as an I2C Slave.
pub struct $Type {
    sda: $pad0,
    scl: $pad1,
    sercom: $SERCOM,
}

impl $Type {
    /// Configures the sercom instance to work as an I2C Slave.
    ///
    pub fn new(
        sercom: $SERCOM,
        pm: &mut PM,
        sda: $pad0,
        scl: $pad1,
        addr: u8,
    ) -> Self {
        // Power up the peripheral bus clock.
        // safe because we're exclusively owning SERCOM
        pm.apbcmask.modify(|_, w| w.$powermask().set_bit());

        unsafe {
            // reset the sercom instance
            sercom.i2cs.ctrla.modify(|_, w| w.swrst().set_bit());
            // wait for reset to complete
            while sercom.i2cs.syncbusy.read().swrst().bit_is_set()
                || sercom.i2cs.ctrla.read().swrst().bit_is_set()
            {}

            // Put the hardware into i2c slave mode
            sercom.i2cs.ctrla.modify(|_, w| w.mode().i2c_slave());
            // wait for configuration to take effect
            while sercom.i2cs.syncbusy.read().enable().bit_is_set() {}

            // enable scl stretch mode. This only triggers an interrupt 
            // _after_ the ack bit, which simplifies things and allows 
            // high speed mode
            sercom.i2cs.ctrla.modify(|_, w| w.sclsm().set_bit());
            // wait for configuration to take effect
            while sercom.i2cs.syncbusy.read().enable().bit_is_set() {}

            // currently, address mask will be all zeros, so only
            // this exact address will be matched
            sercom.i2cs.addr.modify(|_, w| w.addr().bits(addr as u16));
            // wait for configuration to take effect
            while sercom.i2cs.syncbusy.read().enable().bit_is_set() {}

            sercom.i2cs.ctrla.modify(|_, w| w.enable().set_bit());
            // wait for configuration to take effect
            while sercom.i2cs.syncbusy.read().enable().bit_is_set() {}

        }

        Self { sda, scl, sercom }
    }

    /// Breaks the sercom device up into its constituent pins and the SERCOM
    /// instance.  Does not make any changes to power management.
    pub fn free(self) -> ($pad0, $pad1, $SERCOM) {
        (self.sda, self.scl, self.sercom)
    }

    fn i2cs(&self) -> &I2CS {
        unsafe { &self.sercom.i2cs }
    }

    pub fn write(&self, val: u8) {
        unsafe {
            self.i2cs().data.write(|w| w.bits(val));
            self.i2cs().intflag.write(|w| w.amatch().set_bit());
        }
    }
}
        )+
    };
}

i2c!([
    I2CSlave0:
        (
            Sercom0Pad0,
            Sercom0Pad1,
            SERCOM0,
            sercom0_,
            Sercom0CoreClock
        ),
    I2CSlave1:
        (
            Sercom1Pad0,
            Sercom1Pad1,
            SERCOM1,
            sercom1_,
            Sercom1CoreClock
        ),
    I2CSlave2:
        (
            Sercom2Pad0,
            Sercom2Pad1,
            SERCOM2,
            sercom2_,
            Sercom2CoreClock
        ),
    I2CSlave3:
        (
            Sercom3Pad0,
            Sercom3Pad1,
            SERCOM3,
            sercom3_,
            Sercom3CoreClock
        ),
]);

#[cfg(feature = "samd21g18a")]
i2c!([
    I2CSlave4:
        (
            Sercom4Pad0,
            Sercom4Pad1,
            SERCOM4,
            sercom4_,
            Sercom4CoreClock
        ),
    I2CSlave5:
        (
            Sercom5Pad0,
            Sercom5Pad1,
            SERCOM5,
            sercom5_,
            Sercom5CoreClock
        ),
]);

#[derive(Debug)]
pub enum I2CSlaveError {
    Collision,
}
