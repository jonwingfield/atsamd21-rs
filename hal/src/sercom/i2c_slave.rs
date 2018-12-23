// Note: section 7.2.3 shows which pins support I2C Hs mode

use clock;
use sercom::pads::*;
use target_device::sercom0::I2CS;
use target_device::{SERCOM0, SERCOM1, SERCOM2, SERCOM3, PM};
#[cfg(feature = "samd21g18a")]
use target_device::{SERCOM4, SERCOM5};

const BUS_STATE_IDLE: u8 = 1;

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
    tx_buffer: [u8; 128],
    tx_index: usize,
    tx_size: usize,
}

impl $Type {
    /// Configures the sercom instance to work as an I2C Slave.
    ///
    pub fn new(
        clock: &clock::$clock,
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
            while sercom.i2cm.syncbusy.read().swrst().bit_is_set()
                || sercom.i2cm.ctrla.read().swrst().bit_is_set()
            {}

            // Put the hardware into i2c slave mode
            sercom.i2cs.ctrla.modify(|_, w| w.mode().i2c_slave());
            // wait for configuration to take effect
            while sercom.i2cm.syncbusy.read().sysop().bit_is_set() {}
            
            // currently, address mask will be all zeros, so only
            // this exact address will be matched
            sercom.i2cs.addr.modify(|_, w| w.addr().bits(addr as u16));
            sercom.i2cs.addr.modify(|_, w| w.addrmask().bits(0x0));

            // // smart mode enable
            // sercom.i2cs.ctrlb.modify(|_, w| w.smen().set_bit());
            // // wait for configuration to take effect
            // while sercom.i2cs.syncbusy.read().enable().bit_is_set() {}

            // set amode to mask, mask is 0, which means we'll only trigger on exact match
            // sercom.i2cs.ctrlb.modify(|_, w| w.amode().bits(0x0));

            // enable scl stretch mode. This only triggers an interrupt 
            // _after_ the ack bit, which simplifies things and allows 
            // TODO: high speed mode
            // sercom.i2cs.ctrla.modify(|_, w| w.sclsm().set_bit());
            // wait for configuration to take effect
            // while sercom.i2cm.syncbusy.read().sysop().bit_is_set() {}

            // enable interrupts
            sercom.i2cs.intenset.modify(|_, w| w.prec().set_bit());
            sercom.i2cs.intenset.modify(|_, w| w.amatch().set_bit());
            sercom.i2cs.intenset.modify(|_, w| w.drdy().set_bit());
            // wait for configuration to take effect
            while sercom.i2cm.syncbusy.read().sysop().bit_is_set() {}

            // enable
            sercom.i2cm.ctrla.modify(|_, w| w.enable().set_bit());
            // wait for configuration to take effect
            while sercom.i2cm.syncbusy.read().enable().bit_is_set() {}

            // set the bus idle
            sercom
                .i2cm
                .status
                .modify(|_, w| w.busstate().bits(BUS_STATE_IDLE));
            // wait for it to take effect
            while sercom.i2cm.syncbusy.read().sysop().bit_is_set() {}
        }

        Self { sda, scl, sercom, tx_buffer: [0u8; 128], tx_index: 0, tx_size: 0 }
    }

    /// Breaks the sercom device up into its constituent pins and the SERCOM
    /// instance.  Does not make any changes to power management.
    pub fn free(self) -> ($pad0, $pad1, $SERCOM) {
        (self.sda, self.scl, self.sercom)
    }

    fn is_address_match(&self) -> bool {
        self.i2cs().intflag.read().amatch().bit_is_set()
    }

    fn is_data_ready(&self) -> bool {
        self.i2cs().intflag.read().drdy().bit_is_set()
    }

    fn is_master_read(&self) -> bool {
        self.i2cs().status.read().dir().bit_is_set()
    }

    fn is_stop_detected(&self) -> bool {
        self.i2cs().intflag.read().prec().bit_is_set()
    }

    fn prepare_address_match(&self) {
        unsafe {
            self.i2cs().ctrlb.modify(|_, w| w.ackact().clear_bit());
            self.i2cs().ctrlb.modify(|_, w| w.cmd().bits(0x03));
            // self.i2cs().intflag.write(|w| w.amatch().set_bit());
        }
    }

    fn i2cs(&self) -> &I2CS {
        unsafe { &self.sercom.i2cs }
    }

    fn write_byte(&self, val: u8) -> bool {
        unsafe {
            self.i2cs().data.write(|w| w.bits(val));
            return self.i2cs().intflag.read().drdy().bit_is_clear() ||
                self.i2cs().status.read().rxnack().bit_is_set();
        }
    }

    fn write_next(&mut self) -> bool {
        if self.tx_index >= self.tx_size {
            self.write_byte(0xff)
        } else {
            let result = self.write_byte(self.tx_buffer[self.tx_index]);
            self.tx_index += 1;
            result
        }
    }

    /// Queues bytes to be written. The actual writing occurs within 
    /// `service_interrupt` and is an implementation detail
    pub fn write(&mut self, val: &[u8]) {
        self.tx_buffer[0..val.len()].copy_from_slice(val);
        self.tx_index = 0;
        self.tx_size = val.len();
    }

    /// To be called from within the interrupt handler to determine if 
    /// it is appropriate to write bytes
    pub fn is_read_request(&self) -> bool {
        self.is_address_match()
    }

    /// To be called in an the interrupt handler for the corresponding sercom,
    /// this processes the interrupt and does most of the work.
    /// Handles address match, data ready and stop detect interrupts for i2c
    /// slave devices.
    ///
    /// ```no_run
    /// interrupt!(SERCOM3, sercom3);
    /// 
    /// fn sercom3() {
    ///   unsafe { I2CS_DEV.as_mut() }.map(|a| {
    ///     if a.is_read_request() {
    ///       a.write(&[ /* some bytes */ ]);
    ///     }
    ///     a.service_interrupt();
    ///   });
    /// }
    /// ```
    pub fn service_interrupt(&mut self) -> bool {
        if self.is_stop_detected() {
            self.prepare_address_match();
            self.tx_size = 0;
            self.tx_index = 0;
            true
        } else if self.is_address_match() {
            self.prepare_address_match();
            true
        } else if self.is_data_ready() {
            self.write_next()
        } else {
            false 
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
