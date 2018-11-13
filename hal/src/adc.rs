use super::clock::GenericClockController;
use target_device::adc::avgctrl::*;
use target_device::adc::ctrlb::*;
use target_device::adc::inputctrl::*;
use target_device::adc::refctrl::*;
use target_device::adc::{AVGCTRL, CTRLA, CTRLB, INPUTCTRL, REFCTRL};
use target_device::{ADC, PM};

struct State {
    adc: ADC,
}

impl State {
    fn wait_for_sync(&mut self) {
        while self.adc.status.read().syncbusy().bit_is_set() {}
    }

    fn set_calibration(&mut self) {
        let linearity = super::calibration::adc_linearity();
        let bias = super::calibration::adc_biascal();
        self.adc.calib.write(|w| unsafe {
            w.linearity_cal().bits(linearity);
            w.bias_cal().bits(bias)
        });
        self.wait_for_sync();
    }

    fn set_prescaler(&mut self, prescaler: PRESCALERW) {
        self.adc
            .ctrlb
            .write(|w| unsafe { w.prescaler().variant(prescaler) });
        self.wait_for_sync();
    }

    fn set_references(&mut self, adc_ref: REFSELW, gain: GAINW) {
        self.adc
            .refctrl
            .write(|w| unsafe { w.refsel().variant(adc_ref) });
        self.adc.inputctrl.write(|w| unsafe {
            w.muxneg().variant(MUXNEGW::GND);
            w.gain().variant(gain)
        });
        self.wait_for_sync();
    }

    fn set_averaging_mode(&mut self) {
        // TODO: this isn't currently working properly
        self.adc.avgctrl.write(|w| unsafe {
            w.samplenum().variant(SAMPLENUMW::_16);
            // see table 33-3 for values. This is dependant on SAMPLENUM
            w.adjres().bits(0x4)
        });
        self.adc
            .ctrlb
            .modify(|_, w| unsafe { w.ressel().variant(RESSELW::_16BIT) }); // must set per 33.6.7 in datasheet
        self.wait_for_sync();
    }

    fn disable(&mut self) {
        self.adc.ctrla.write(|w| w.enable().clear_bit());
        // wait for synchronization
        self.wait_for_sync();
    }

    fn enable(&mut self) {
        self.adc.ctrla.write(|w| w.enable().set_bit());
        // wait for synchronization
        self.wait_for_sync();
    }

    fn read(&mut self, neg: MUXNEGW, pos: MUXPOSW) -> u16 {
        self.wait_for_sync();
        // // TEMP for current project
        // match pos {
        //     MUXPOSW::PIN3 => self.set_references(REFSELW::INTVCC1, GAINW::_1X),
        //     _ => self.set_references(REFSELW::INTVCC1, GAINW::DIV2),
        // }
        self.adc.inputctrl.modify(|_, w| w.muxpos().variant(pos));
        self.wait_for_sync();
        // self.set_references(REFSELW::INTVCC1, GAINW::DIV2);
        self.enable();
        self.adc.swtrig.write(|w| w.start().set_bit());
        self.adc.intflag.write(|w| w.resrdy().set_bit());
        self.wait_for_sync();
        // start another conversion. First one after updating the mux must be thrown
        // away
        self.adc.swtrig.write(|w| w.start().set_bit());
        while self.adc.intflag.read().resrdy().bit_is_clear() {
            // wait for conversion to complete
        }
        let result = self.adc.result.read().bits();
        self.disable();
        result
    }
}

fn enable_adc_apb(pm: &mut PM) {
    pm.apbcmask.modify(|_, w| w.adc_().set_bit());
}

pub struct Adc {
    state: State,
}

impl Adc {
    pub fn new(clocks: &mut GenericClockController, pm: &mut PM, adc: ADC) -> Self {
        let mut state = State { adc };

        // power up the adc
        enable_adc_apb(pm);
        //
        // ** Steps to Enable the ADC **
        // a. set IO PORT configuration
        // b. load BIAS_CAL and LINEARITY_CAL from the NVM Software calibration area
        // into the ADC Calibration register (CALIB)
        // c. enable ADC in PM
        state.set_calibration();

        let glck0 = clocks.gclk0();
        // enable the clock generator for the adc and get the singleton for it (unwrap
        // fails for subsequent requests)
        // TODO: store this somewhere for future reference
        clocks.adc(&glck0).unwrap();
        // wait for synchronization of registers between the clock domains (see manual)
        state.wait_for_sync();

        // INTVCC1 = VDDANA/2, so divide the gain by two
        state.set_references(REFSELW::INTVCC1, GAINW::DIV2);
        state.set_prescaler(PRESCALERW::DIV32);
        // state.set_averaging_mode();

        // 1. Choose async clock source and enable. SYSCTRL.GCLK_ADC needs to be
        // selected and enabled
        //
        // 2. Set ADC Reference (internal 1v, etc)
        // 2. Write a 1 to ADC::CTRLA.ENABLE !!
        // 3. Setup InputCtrl register to use correct input
        // 3. Setup Averaging in AVGCTRL.SAMPLENUM = 0x4 (16 samples) 0x5 (32 Samples)
        // 0x6 (64 samples)
        // 4. Use RESRDY interrupt to know when conversion is ready
        // 5. Read value from RESULT register !!
        //
        // !! = requires synchronization
        // INTFLAG.RESRDY (Result Ready interrupt)

        Adc { state }
    }

    // TODO: pin-based interface
    pub fn read_sync(&mut self, pos: u8) -> u16 {
        self.state.read(
            MUXNEGW::GND,
            match pos {
                0 => MUXPOSW::PIN0,
                1 => MUXPOSW::PIN2,
                2 => MUXPOSW::PIN3,
                _ => MUXPOSW::PIN0,
            },
        )
    }
}

/* 14.4 Enabling a peripheral
 *
 * In order to enable a peripheral that is clocked by a Generic Clock, the following parts of the system needs
to be configured:
* A running Clock Source.
* A clock from the Generic Clock Generator must be configured to use one of the running Clock
Sources, and the Generator must be enabled.
* The Generic Clock Multiplexer that provides the Generic Clock signal to the peripheral must be
configured to use a running Generic Clock Generator, and the Generic Clock must be enabled.
* The user interface of the peripheral needs to be unmasked in the PM. If this is not done the
peripheral registers will read all 0?s and any writing attempts to the peripheral will be discarded.
*/
fn enable_adc_peripheral() {}
