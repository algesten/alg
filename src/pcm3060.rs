//! Driver and sample-buffer helpers for the TI PCM3060 stereo ADC/DAC codec.
//!
//! The register API targets the PCM3060 serial control port over I2C using the
//! `embedded-hal` 0.2 blocking `WriteIter` trait, because that is what the
//! current STM32F4 HAL exposes for I2C. The audio stream itself is not carried
//! by this driver; PCM3060 audio samples move over I2S and DMA, so this module
//! provides zero-copy views over caller-owned DMA buffers instead.
//!
//! # Configure the codec over I2C
//!
//! ```no_run
//! use alg::pcm3060::{
//!     AddressSelect, AudioFormat, Config, DacOutput, HighPass, Pcm3060, Power,
//! };
//! use embedded_hal::blocking::i2c::WriteIter;
//!
//! struct I2c;
//!
//! impl WriteIter for I2c {
//!     type Error = ();
//!
//!     fn write<B>(&mut self, _addr: u8, _bytes: B) -> Result<(), Self::Error>
//!     where
//!         B: IntoIterator<Item = u8>,
//!     {
//!         Ok(())
//!     }
//! }
//!
//! let i2c = I2c;
//! let mut codec = Pcm3060::new(i2c, AddressSelect::Low);
//!
//! let mut config = Config::default();
//! config.control.dac_output = DacOutput::SingleEnded;
//! config.control.adc_power = Power::Normal;
//! config.control.dac_power = Power::Normal;
//! config.adc.control.high_pass = HighPass::Bypassed;
//! config.adc.interface.format = AudioFormat::I2s24;
//! config.dac.interface.format = AudioFormat::I2s24;
//!
//! codec.configure(&config).unwrap();
//! ```
//!
//! # Configure with clock validation and power-save release delay
//!
//! ```no_run
//! use alg::pcm3060::{
//!     AddressSelect, AudioFormat, Config, DacOutput, HighPass, Pcm3060,
//! };
//! use embedded_hal::blocking::delay::DelayUs;
//! use embedded_hal::blocking::i2c::WriteIter;
//!
//! struct Delay;
//! impl DelayUs<u32> for Delay {
//!     fn delay_us(&mut self, _us: u32) {}
//! }
//!
//! # struct I2c;
//! # impl WriteIter for I2c {
//! #     type Error = ();
//! #     fn write<B>(&mut self, _addr: u8, _bytes: B) -> Result<(), Self::Error>
//! #     where
//! #         B: IntoIterator<Item = u8>,
//! #     {
//! #         Ok(())
//! #     }
//! # }
//! const FS_HZ: u32 = 48_000;
//! const SCKI_HZ: u32 = 12_288_000;
//!
//! let setup = Config::builder()
//!     .dac_output(DacOutput::SingleEnded)
//!     .adc_high_pass(HighPass::Bypassed)
//!     .audio_format(AudioFormat::I2s24)
//!     .clocks(FS_HZ, SCKI_HZ, SCKI_HZ)
//!     .unwrap()
//!     .powered_up()
//!     .build();
//!
//! let mut codec = Pcm3060::new(I2c, AddressSelect::Low);
//! let mut delay = Delay;
//! codec.configure_validated(&setup, &mut delay).unwrap();
//! ```
//!
//! # External reset, power-down, and zero-flag pins
//!
//! ```no_run
//! use alg::pcm3060::{ControlPins, ZeroFlagPins, ZeroFlagPolarity};
//! use embedded_hal::blocking::delay::DelayUs;
//! use embedded_hal::digital::v2::{InputPin, OutputPin};
//!
//! struct Delay;
//! impl DelayUs<u32> for Delay {
//!     fn delay_us(&mut self, _us: u32) {}
//! }
//!
//! struct Out;
//! impl OutputPin for Out {
//!     type Error = ();
//!     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
//!     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
//! }
//!
//! struct In;
//! impl InputPin for In {
//!     type Error = ();
//!     fn is_high(&self) -> Result<bool, Self::Error> { Ok(true) }
//!     fn is_low(&self) -> Result<bool, Self::Error> { Ok(false) }
//! }
//!
//! let mut delay = Delay;
//! let mut control = ControlPins::new(Out, Out);
//! control.power_up().unwrap();
//! control.reset_pulse(10, &mut delay).unwrap();
//!
//! let flags = ZeroFlagPins::new(In, In);
//! let _dac_is_silent = flags.both_zero(ZeroFlagPolarity::HighForZero).unwrap();
//! ```
//!
//! # Zero-copy DMA buffer processing pattern
//!
//! The DMA/HAL layer should own buffers while the peripheral is using them.
//! When a transfer completes, exchange a processed/free buffer for the
//! completed buffer, process the completed input directly into the completed
//! output, and submit those buffers back for a future DMA cycle.
//!
//! ```no_run
//! use alg::pcm3060::{Pcm24In, Pcm24Out, StereoF32};
//!
//! const WORDS: usize = 512;
//! type Buffer = &'static mut [u16; WORDS];
//!
//! struct DmaTransfer {
//!     active: Buffer,
//! }
//!
//! impl DmaTransfer {
//!     fn next_transfer(&mut self, mut replacement: Buffer) -> Buffer {
//!         core::mem::swap(&mut self.active, &mut replacement);
//!         replacement
//!     }
//! }
//!
//! struct ReadyPair {
//!     tx: Buffer,
//!     rx: Buffer,
//! }
//!
//! fn dma_complete_isr(
//!     tx_dma: &mut DmaTransfer,
//!     rx_dma: &mut DmaTransfer,
//!     ready: ReadyPair,
//! ) -> ReadyPair {
//!     let completed_tx = tx_dma.next_transfer(ready.tx);
//!     let completed_rx = rx_dma.next_transfer(ready.rx);
//!     ReadyPair {
//!         tx: completed_tx,
//!         rx: completed_rx,
//!     }
//! }
//!
//! fn process_completed_pair(mut pair: ReadyPair) -> ReadyPair {
//!     let input = Pcm24In::new(&pair.rx[..]).unwrap();
//!     let frames = input.frames().map(|mut frame: StereoF32| {
//!         frame.left *= 0.75;
//!         frame.right *= 0.75;
//!         frame
//!     });
//!
//!     Pcm24Out::new(&mut pair.tx[..]).unwrap().write_frames(frames);
//!     pair
//! }
//! ```
//!
//! The important invariant is that the DMA transfer object owns active buffers,
//! and software only receives a buffer after the DMA API returns it.
//!
//! For [`AudioFormat::RightJustified16`](crate::pcm3060::AudioFormat::RightJustified16),
//! use [`Pcm16In`] and [`Pcm16Out`] with two `u16` words per stereo frame.

use core::marker::PhantomData;
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::blocking::i2c::WriteIter;
use embedded_hal::digital::v2::{InputPin, OutputPin};

const BASE_ADDR: u8 = 0b100_0110;

#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
#[repr(u8)]
enum Register {
    Control = 0x40,
    DacAttenuationLeft = 0x41,
    DacAttenuationRight = 0x42,
    DacInterface = 0x43,
    DacControl = 0x44,
    DacFilter = 0x45,
    AdcAttenuationLeft = 0x46,
    AdcAttenuationRight = 0x47,
    AdcInterface = 0x48,
    AdcControl = 0x49,
}

/// PCM3060 I2C control-port driver.
///
/// This type owns the I2C peripheral until [`release`](Self::release) is called.
pub struct Pcm3060<I2C> {
    i2c: I2C,
    addr: u8,
}

impl<I2C> Pcm3060<I2C> {
    /// Create a driver for a PCM3060 on the selected 7-bit I2C address.
    pub fn new(i2c: I2C, address_select: AddressSelect) -> Self {
        Pcm3060 {
            i2c,
            addr: BASE_ADDR | address_select.bit(),
        }
    }

    /// Return the owned I2C peripheral.
    pub fn release(self) -> I2C {
        self.i2c
    }
}

impl<I2C, E> Pcm3060<I2C>
where
    I2C: WriteIter<Error = E>,
{
    /// Write a complete typed configuration to all PCM3060 mode-control registers.
    ///
    /// This writes registers `0x40..0x49` in increasing register order. If you
    /// need the recommended power-save release delay, use
    /// [`configure_with_power_release_delay`](Self::configure_with_power_release_delay).
    pub fn configure(&mut self, config: &Config) -> Result<(), Error<E>> {
        self.write_control(config.control)?;
        self.write_dac_attenuation_left(config.dac.attenuation_left)?;
        self.write_dac_attenuation_right(config.dac.attenuation_right)?;
        self.write_dac_interface(config.dac.interface)?;
        self.write_dac_control(config.dac.control)?;
        self.write_dac_filter(config.dac.filter)?;
        self.write_adc_attenuation_left(config.adc.attenuation_left)?;
        self.write_adc_attenuation_right(config.adc.attenuation_right)?;
        self.write_adc_interface(config.adc.interface)?;
        self.write_adc_control(config.adc.control)?;
        Ok(())
    }

    /// Configure the codec while powered down, then release power-save and wait.
    ///
    /// The datasheet recommends changing clocks and audio-interface mode while
    /// powered down. This helper writes `config` with both ADC and DAC forced to
    /// power-save, writes the requested final power state, then waits at least
    /// `2048 / min(adc_fs_hz, dac_fs_hz)` microseconds if either converter was
    /// released to normal operation.
    pub fn configure_with_power_release_delay<D>(
        &mut self,
        config: &Config,
        adc_fs_hz: u32,
        dac_fs_hz: u32,
        delay: &mut D,
    ) -> Result<(), ConfigureError<E>>
    where
        D: DelayUs<u32>,
    {
        validate_no_reset_commands(config).map_err(ConfigureError::InvalidConfig)?;
        validate_power_release_sample_rates(config, adc_fs_hz, dac_fs_hz)
            .map_err(ConfigureError::InvalidConfig)?;

        let mut powered_down = *config;
        powered_down.control.adc_power = Power::PowerSave;
        powered_down.control.dac_power = Power::PowerSave;
        self.configure(&powered_down)
            .map_err(ConfigureError::Write)?;

        self.write_control(config.control)
            .map_err(ConfigureError::Write)?;

        if config.control.adc_power == Power::Normal || config.control.dac_power == Power::Normal {
            let fs_hz = min_nonzero(adc_fs_hz, dac_fs_hz);
            delay.delay_us(power_change_delay_us(fs_hz));
        }

        Ok(())
    }

    /// Configure a completed phase-validated setup and wait after power release.
    ///
    /// Prefer this over [`configure_with_power_release_delay`](Self::configure_with_power_release_delay)
    /// when constructing new setups. The validated setup carries the sample
    /// rates needed for the datasheet `2048 / fS` power-release delay, so the
    /// call site cannot pass mismatched timing values.
    pub fn configure_validated<D>(
        &mut self,
        setup: &ValidatedConfig,
        delay: &mut D,
    ) -> Result<(), Error<E>>
    where
        D: DelayUs<u32>,
    {
        let mut powered_down = setup.config;
        powered_down.control.adc_power = Power::PowerSave;
        powered_down.control.dac_power = Power::PowerSave;
        self.configure(&powered_down)?;

        self.write_control(setup.config.control)?;

        if setup.config.control.adc_power == Power::Normal
            || setup.config.control.dac_power == Power::Normal
        {
            if let Some(clocks) = setup.clocks {
                delay.delay_us(power_change_delay_us(min_nonzero(
                    clocks.adc_fs_hz,
                    clocks.dac_fs_hz,
                )));
            }
        }

        Ok(())
    }

    /// Configure the codec for the current two4eight bring-up path:
    ///
    /// - DAC output single-ended while still in power-save.
    /// - ADC high-pass filter disabled.
    /// - DAC and ADC released from power-save.
    pub fn configure_single_ended_no_adc_high_pass(&mut self) -> Result<(), Error<E>> {
        let mut config = Config::default();
        config.control.dac_output = DacOutput::SingleEnded;
        config.adc.control.high_pass = HighPass::Bypassed;

        self.write_control(config.control)?;
        self.write_adc_control(config.adc.control)?;

        config.control.adc_power = Power::Normal;
        config.control.dac_power = Power::Normal;
        self.write_control(config.control)
    }

    /// Write global reset, power-save, and DAC output mode control.
    pub fn write_control(&mut self, control: Control) -> Result<(), Error<E>> {
        self.write_register(Operation::Control, Register::Control, control.bits())
    }

    /// Write DAC left-channel attenuation.
    pub fn write_dac_attenuation_left(
        &mut self,
        attenuation: DacAttenuation,
    ) -> Result<(), Error<E>> {
        self.write_register(
            Operation::DacAttenuationLeft,
            Register::DacAttenuationLeft,
            attenuation.0,
        )
    }

    /// Write DAC right-channel attenuation.
    pub fn write_dac_attenuation_right(
        &mut self,
        attenuation: DacAttenuation,
    ) -> Result<(), Error<E>> {
        self.write_register(
            Operation::DacAttenuationRight,
            Register::DacAttenuationRight,
            attenuation.0,
        )
    }

    /// Write DAC audio interface configuration.
    pub fn write_dac_interface(&mut self, interface: DacInterface) -> Result<(), Error<E>> {
        self.write_register(
            Operation::DacInterface,
            Register::DacInterface,
            interface.bits(),
        )
    }

    /// Write DAC oversampling, phase, and mute configuration.
    pub fn write_dac_control(&mut self, control: DacControl) -> Result<(), Error<E>> {
        self.write_register(Operation::DacControl, Register::DacControl, control.bits())
    }

    /// Write DAC filter, de-emphasis, and zero-flag configuration.
    pub fn write_dac_filter(&mut self, filter: DacFilter) -> Result<(), Error<E>> {
        self.write_register(Operation::DacFilter, Register::DacFilter, filter.bits())
    }

    /// Write ADC left-channel attenuation.
    pub fn write_adc_attenuation_left(
        &mut self,
        attenuation: AdcAttenuation,
    ) -> Result<(), Error<E>> {
        self.write_register(
            Operation::AdcAttenuationLeft,
            Register::AdcAttenuationLeft,
            attenuation.0,
        )
    }

    /// Write ADC right-channel attenuation.
    pub fn write_adc_attenuation_right(
        &mut self,
        attenuation: AdcAttenuation,
    ) -> Result<(), Error<E>> {
        self.write_register(
            Operation::AdcAttenuationRight,
            Register::AdcAttenuationRight,
            attenuation.0,
        )
    }

    /// Write ADC audio interface configuration.
    pub fn write_adc_interface(&mut self, interface: AdcInterface) -> Result<(), Error<E>> {
        self.write_register(
            Operation::AdcInterface,
            Register::AdcInterface,
            interface.bits(),
        )
    }

    /// Write ADC zero-cross, high-pass filter, phase, and mute configuration.
    pub fn write_adc_control(&mut self, control: AdcControl) -> Result<(), Error<E>> {
        self.write_register(Operation::AdcControl, Register::AdcControl, control.bits())
    }

    fn write_register(
        &mut self,
        operation: Operation,
        register: Register,
        value: u8,
    ) -> Result<(), Error<E>> {
        self.i2c
            .write(self.addr, [register as u8, value])
            .map_err(|source| Error { operation, source })
    }
}

/// Typed write operation that failed on the PCM3060 control port.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum Operation {
    /// Global reset, power-save, and DAC output mode control.
    Control,
    /// DAC left-channel attenuation.
    DacAttenuationLeft,
    /// DAC right-channel attenuation.
    DacAttenuationRight,
    /// DAC audio interface configuration.
    DacInterface,
    /// DAC oversampling, phase, and mute configuration.
    DacControl,
    /// DAC filter, de-emphasis, and zero-flag configuration.
    DacFilter,
    /// ADC left-channel attenuation.
    AdcAttenuationLeft,
    /// ADC right-channel attenuation.
    AdcAttenuationRight,
    /// ADC audio interface configuration.
    AdcInterface,
    /// ADC zero-cross, high-pass filter, phase, and mute configuration.
    AdcControl,
}

/// Complete PCM3060 mode-control configuration.
///
/// Defaults match the datasheet reset values: ADC and DAC in power-save,
/// I2S/slave interfaces, DAC differential output, no attenuation, no soft mute.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct Config {
    /// Register `0x40`: global reset/power/output-mode control.
    pub control: Control,
    /// DAC-related registers `0x41..0x45`.
    pub dac: DacConfig,
    /// ADC-related registers `0x46..0x49`.
    pub adc: AdcConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            control: Control::default(),
            dac: DacConfig::default(),
            adc: AdcConfig::default(),
        }
    }
}

impl Config {
    /// Start a validating builder from the datasheet-reset configuration.
    pub fn builder() -> ConfigBuilder<Draft> {
        ConfigBuilder::new()
    }

    /// Validate this configuration against optional board clock information.
    pub fn validate(&self, clocks: ClockConfig) -> Result<(), ConfigError> {
        validate_no_reset_commands(self)?;
        validate_clocks(self, clocks)
    }
}

/// Optional clock facts used to validate a low-level PCM3060 configuration.
#[derive(Clone, Copy, Debug, Default, defmt::Format, PartialEq, Eq)]
pub struct ClockConfig {
    /// ADC sample rate in hertz.
    pub adc_fs_hz: Option<u32>,
    /// DAC sample rate in hertz.
    pub dac_fs_hz: Option<u32>,
    /// Frequency applied to the selected ADC system-clock input.
    pub adc_scki_hz: Option<u32>,
    /// Frequency applied to the selected DAC system-clock input.
    pub dac_scki_hz: Option<u32>,
}

impl ClockConfig {
    /// Create an empty clock configuration.
    pub fn new() -> Self {
        ClockConfig::default()
    }

    /// Set the ADC sample rate.
    pub fn adc_sample_rate(mut self, fs_hz: u32) -> Self {
        self.adc_fs_hz = Some(fs_hz);
        self
    }

    /// Set the DAC sample rate.
    pub fn dac_sample_rate(mut self, fs_hz: u32) -> Self {
        self.dac_fs_hz = Some(fs_hz);
        self
    }

    /// Set both ADC and DAC sample rates to the same value.
    pub fn sample_rate(mut self, fs_hz: u32) -> Self {
        self.adc_fs_hz = Some(fs_hz);
        self.dac_fs_hz = Some(fs_hz);
        self
    }

    /// Set the ADC system-clock input frequency.
    pub fn adc_scki(mut self, scki_hz: u32) -> Self {
        self.adc_scki_hz = Some(scki_hz);
        self
    }

    /// Set the DAC system-clock input frequency.
    pub fn dac_scki(mut self, scki_hz: u32) -> Self {
        self.dac_scki_hz = Some(scki_hz);
        self
    }

    /// Set both ADC and DAC system-clock input frequencies to the same value.
    pub fn scki(mut self, scki_hz: u32) -> Self {
        self.adc_scki_hz = Some(scki_hz);
        self.dac_scki_hz = Some(scki_hz);
        self
    }
}

/// Draft setup phase.
///
/// Register preferences can be edited in this phase. A powered-up validated
/// configuration cannot be built until the setup has entered [`Clocked`].
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct Draft;

/// Clocked setup phase.
///
/// The sample-rate and system-clock facts have been validated. Power-up is now
/// available.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct Clocked;

/// Ready setup phase.
///
/// Final power state has been selected and the setup can be built infallibly.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct Ready;

/// Validated clock facts for a PCM3060 configuration.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct ValidatedClocks {
    /// ADC sample rate in hertz.
    pub adc_fs_hz: u32,
    /// DAC sample rate in hertz.
    pub dac_fs_hz: u32,
    /// Frequency applied to the selected ADC system-clock input.
    pub adc_scki_hz: u32,
    /// Frequency applied to the selected DAC system-clock input.
    pub dac_scki_hz: u32,
    /// Validated ADC `SCKI/fS` ratio.
    pub adc_ratio: AdcClockRatio,
    /// Validated DAC `SCKI/fS` ratio.
    pub dac_ratio: DacClockRatio,
}

/// A PCM3060 configuration that completed the setup phases.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct ValidatedConfig {
    config: Config,
    clocks: Option<ValidatedClocks>,
}

impl ValidatedConfig {
    /// Return the validated register configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Return the validated clock facts, if this setup supplied clocks.
    pub fn clocks(&self) -> Option<ValidatedClocks> {
        self.clocks
    }
}

/// Builder for phase-validated PCM3060 mode-control configurations.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct ConfigBuilder<Phase = Draft> {
    config: Config,
    clocks: Option<ValidatedClocks>,
    _phase: PhantomData<Phase>,
}

impl ConfigBuilder<Draft> {
    /// Start from the datasheet-reset configuration.
    pub fn new() -> Self {
        ConfigBuilder {
            config: Config::default(),
            clocks: None,
            _phase: PhantomData,
        }
    }

    /// Start from an existing configuration.
    pub fn from_config(config: Config) -> Result<Self, ConfigError> {
        validate_no_reset_commands(&config)?;
        Ok(ConfigBuilder {
            config,
            clocks: None,
            _phase: PhantomData,
        })
    }

    /// Validate identical ADC/DAC sample rates and system clocks, entering the clocked phase.
    pub fn clocks(
        self,
        sample_rate_hz: u32,
        adc_scki_hz: u32,
        dac_scki_hz: u32,
    ) -> Result<ConfigBuilder<Clocked>, ConfigError> {
        self.adc_dac_clocks(sample_rate_hz, sample_rate_hz, adc_scki_hz, dac_scki_hz)
    }

    /// Validate ADC/DAC sample rates and system clocks, entering the clocked phase.
    pub fn adc_dac_clocks(
        self,
        adc_fs_hz: u32,
        dac_fs_hz: u32,
        adc_scki_hz: u32,
        dac_scki_hz: u32,
    ) -> Result<ConfigBuilder<Clocked>, ConfigError> {
        validate_no_reset_commands(&self.config)?;

        let adc_ratio =
            validate_adc_clock(adc_scki_hz, adc_fs_hz).map_err(ConfigError::AdcClock)?;
        let dac_ratio =
            validate_dac_clock(dac_scki_hz, dac_fs_hz).map_err(ConfigError::DacClock)?;

        validate_adc_master_ratio(&self.config, adc_ratio)?;
        validate_dac_master_ratio(&self.config, dac_ratio)?;

        Ok(ConfigBuilder {
            config: self.config,
            clocks: Some(ValidatedClocks {
                adc_fs_hz,
                dac_fs_hz,
                adc_scki_hz,
                dac_scki_hz,
                adc_ratio,
                dac_ratio,
            }),
            _phase: PhantomData,
        })
    }

    /// Build a validated configuration that remains in power-save.
    ///
    /// Clock facts are not required because no converter is released.
    pub fn build_power_save(mut self) -> ValidatedConfig {
        self.config.control.adc_power = Power::PowerSave;
        self.config.control.dac_power = Power::PowerSave;
        ValidatedConfig {
            config: self.config,
            clocks: None,
        }
    }

    /// Set the ADC audio-interface mode.
    pub fn adc_mode(mut self, mode: AdcMode) -> Self {
        self.config.adc.interface.mode = mode;
        self
    }

    /// Set the DAC audio-interface mode.
    pub fn dac_mode(mut self, mode: DacMode) -> Self {
        self.config.dac.interface.mode = mode;
        self
    }
}

impl<Phase> ConfigBuilder<Phase> {
    fn into_phase<Next>(self) -> ConfigBuilder<Next> {
        ConfigBuilder {
            config: self.config,
            clocks: self.clocks,
            _phase: PhantomData,
        }
    }

    /// Set the DAC analog output mode.
    pub fn dac_output(mut self, output: DacOutput) -> Self {
        self.config.control.dac_output = output;
        self
    }

    /// Set both ADC and DAC serial audio formats.
    pub fn audio_format(mut self, format: AudioFormat) -> Self {
        self.config.adc.interface.format = format;
        self.config.dac.interface.format = format;
        self
    }

    /// Set the ADC serial audio format.
    pub fn adc_audio_format(mut self, format: AudioFormat) -> Self {
        self.config.adc.interface.format = format;
        self
    }

    /// Set the DAC serial audio format.
    pub fn dac_audio_format(mut self, format: AudioFormat) -> Self {
        self.config.dac.interface.format = format;
        self
    }

    /// Set whether the ADC high-pass filter is enabled or bypassed.
    pub fn adc_high_pass(mut self, high_pass: HighPass) -> Self {
        self.config.adc.control.high_pass = high_pass;
        self
    }

    /// Set the DAC filter and zero-flag configuration.
    pub fn dac_filter(mut self, filter: DacFilter) -> Self {
        self.config.dac.filter = filter;
        self
    }

    /// Set ADC and DAC attenuation registers.
    pub fn attenuation(
        mut self,
        adc_left: AdcAttenuation,
        adc_right: AdcAttenuation,
        dac_left: DacAttenuation,
        dac_right: DacAttenuation,
    ) -> Self {
        self.config.adc.attenuation_left = adc_left;
        self.config.adc.attenuation_right = adc_right;
        self.config.dac.attenuation_left = dac_left;
        self.config.dac.attenuation_right = dac_right;
        self
    }
}

impl ConfigBuilder<Clocked> {
    /// Release both ADC and DAC from power-save, entering the ready phase.
    pub fn powered_up(mut self) -> ConfigBuilder<Ready> {
        self.config.control.adc_power = Power::Normal;
        self.config.control.dac_power = Power::Normal;
        self.into_phase()
    }

    /// Keep both ADC and DAC in power-save, entering the ready phase.
    pub fn keep_power_save(mut self) -> ConfigBuilder<Ready> {
        self.config.control.adc_power = Power::PowerSave;
        self.config.control.dac_power = Power::PowerSave;
        self.into_phase()
    }
}

impl ConfigBuilder<Ready> {
    /// Build the completed setup infallibly.
    pub fn build(self) -> ValidatedConfig {
        ValidatedConfig {
            config: self.config,
            clocks: self.clocks,
        }
    }
}

impl Default for ConfigBuilder<Draft> {
    fn default() -> Self {
        ConfigBuilder::new()
    }
}

/// Error returned by validated configuration construction.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum ConfigError {
    /// A persistent configuration contained a mode-control reset command.
    ResetCommandInConfig,
    /// The supplied ADC system clock/sample-rate pair is invalid.
    AdcClock(ClockError),
    /// The supplied DAC system clock/sample-rate pair is invalid.
    DacClock(ClockError),
    /// The ADC master-mode ratio does not match the supplied ADC clock ratio.
    AdcMasterRatioMismatch {
        /// Ratio implied by the selected ADC master mode.
        mode_ratio: u32,
        /// Ratio calculated from the supplied clock facts.
        clock_ratio: u32,
    },
    /// The DAC master-mode ratio does not match the supplied DAC clock ratio.
    DacMasterRatioMismatch {
        /// Ratio implied by the selected DAC master mode.
        mode_ratio: u32,
        /// Ratio calculated from the supplied clock facts.
        clock_ratio: u32,
    },
}

/// Global control register fields for register `0x40`.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct Control {
    /// Mode-control register reset bit (`MRST`).
    pub mode_control_reset: Reset,
    /// System resynchronization reset bit (`SRST`).
    pub system_reset: Reset,
    /// ADC power-save control (`ADPSV`).
    pub adc_power: Power,
    /// DAC power-save control (`DAPSV`).
    pub dac_power: Power,
    /// DAC analog output pin configuration (`S/E`).
    pub dac_output: DacOutput,
}

impl Control {
    fn bits(self) -> u8 {
        (self.mode_control_reset.bit() << 7)
            | (self.system_reset.bit() << 6)
            | (self.adc_power.power_save_bit() << 5)
            | (self.dac_power.power_save_bit() << 4)
            | self.dac_output.bit()
    }
}

impl Default for Control {
    fn default() -> Self {
        Control {
            mode_control_reset: Reset::Normal,
            system_reset: Reset::Normal,
            adc_power: Power::PowerSave,
            dac_power: Power::PowerSave,
            dac_output: DacOutput::Differential,
        }
    }
}

/// DAC configuration registers.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct DacConfig {
    /// DAC left-channel digital attenuation, register `0x41`.
    pub attenuation_left: DacAttenuation,
    /// DAC right-channel digital attenuation, register `0x42`.
    pub attenuation_right: DacAttenuation,
    /// DAC audio interface setup, register `0x43`.
    pub interface: DacInterface,
    /// DAC oversampling, phase, and mute setup, register `0x44`.
    pub control: DacControl,
    /// DAC filter, de-emphasis, and zero-flag setup, register `0x45`.
    pub filter: DacFilter,
}

impl Default for DacConfig {
    fn default() -> Self {
        DacConfig {
            attenuation_left: DacAttenuation::ZERO_DB,
            attenuation_right: DacAttenuation::ZERO_DB,
            interface: DacInterface::default(),
            control: DacControl::default(),
            filter: DacFilter::default(),
        }
    }
}

/// DAC audio interface fields for register `0x43`.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct DacInterface {
    /// Which clock/audio-interface pins feed the DAC (`CSEL2`).
    pub clock: DacClock,
    /// DAC slave or master mode (`M/S2[2:0]`).
    pub mode: DacMode,
    /// DAC serial audio data format (`FMT2[1:0]`).
    pub format: AudioFormat,
}

impl DacInterface {
    fn bits(self) -> u8 {
        (self.clock.bit() << 7) | (self.mode.bits() << 4) | self.format.bits()
    }
}

impl Default for DacInterface {
    fn default() -> Self {
        DacInterface {
            clock: DacClock::DacPins,
            mode: DacMode::Slave,
            format: AudioFormat::I2s24,
        }
    }
}

/// DAC control fields for register `0x44`.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct DacControl {
    /// DAC oversampling-rate control (`OVER`).
    pub oversampling: DacOversampling,
    /// DAC output phase (`DREV2`).
    pub phase: Phase,
    /// DAC left-channel soft mute (`MUT21`).
    pub mute_left: Mute,
    /// DAC right-channel soft mute (`MUT22`).
    pub mute_right: Mute,
}

impl DacControl {
    fn bits(self) -> u8 {
        (self.oversampling.bit() << 6)
            | (self.phase.bit() << 2)
            | (self.mute_right.bit() << 1)
            | self.mute_left.bit()
    }
}

impl Default for DacControl {
    fn default() -> Self {
        DacControl {
            oversampling: DacOversampling::Low,
            phase: Phase::Normal,
            mute_left: Mute::Disabled,
            mute_right: Mute::Disabled,
        }
    }
}

/// DAC filter and zero-detect fields for register `0x45`.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct DacFilter {
    /// DAC digital filter rolloff (`FLT`).
    pub rolloff: FilterRolloff,
    /// Sampling-rate selection for de-emphasis (`DMF[1:0]`).
    pub deemphasis_rate: DeemphasisRate,
    /// DAC digital de-emphasis enable (`DMC`).
    pub deemphasis: Deemphasis,
    /// Zero-flag output polarity (`ZREV`).
    pub zero_flag_polarity: ZeroFlagPolarity,
    /// Zero-flag output function (`AZRO`).
    pub zero_flag_function: ZeroFlagFunction,
}

impl DacFilter {
    fn bits(self) -> u8 {
        (self.rolloff.bit() << 7)
            | (self.deemphasis_rate.bits() << 5)
            | (self.deemphasis.bit() << 4)
            | (self.zero_flag_polarity.bit() << 1)
            | self.zero_flag_function.bit()
    }
}

impl Default for DacFilter {
    fn default() -> Self {
        DacFilter {
            rolloff: FilterRolloff::Sharp,
            deemphasis_rate: DeemphasisRate::Hz44100,
            deemphasis: Deemphasis::Disabled,
            zero_flag_polarity: ZeroFlagPolarity::HighForZero,
            zero_flag_function: ZeroFlagFunction::Independent,
        }
    }
}

/// ADC configuration registers.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct AdcConfig {
    /// ADC left-channel digital attenuation, register `0x46`.
    pub attenuation_left: AdcAttenuation,
    /// ADC right-channel digital attenuation, register `0x47`.
    pub attenuation_right: AdcAttenuation,
    /// ADC audio interface setup, register `0x48`.
    pub interface: AdcInterface,
    /// ADC zero-cross, high-pass filter, phase, and mute setup, register `0x49`.
    pub control: AdcControl,
}

impl Default for AdcConfig {
    fn default() -> Self {
        AdcConfig {
            attenuation_left: AdcAttenuation::ZERO_DB,
            attenuation_right: AdcAttenuation::ZERO_DB,
            interface: AdcInterface::default(),
            control: AdcControl::default(),
        }
    }
}

/// ADC audio interface fields for register `0x48`.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct AdcInterface {
    /// Which clock/audio-interface pins feed the ADC (`CSEL1`).
    pub clock: AdcClock,
    /// ADC slave or master mode (`M/S1[2:0]`).
    pub mode: AdcMode,
    /// ADC serial audio data format (`FMT1[1:0]`).
    pub format: AudioFormat,
}

impl AdcInterface {
    fn bits(self) -> u8 {
        (self.clock.bit() << 7) | (self.mode.bits() << 4) | self.format.bits()
    }
}

impl Default for AdcInterface {
    fn default() -> Self {
        AdcInterface {
            clock: AdcClock::AdcPins,
            mode: AdcMode::Slave,
            format: AudioFormat::I2s24,
        }
    }
}

/// ADC control fields for register `0x49`.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct AdcControl {
    /// ADC attenuation/mute zero-cross timing (`ZCDD`).
    pub zero_cross: ZeroCross,
    /// ADC high-pass filter bypass (`BYP`).
    pub high_pass: HighPass,
    /// ADC input phase (`DREV1`).
    pub phase: Phase,
    /// ADC left-channel soft mute (`MUT11`).
    pub mute_left: Mute,
    /// ADC right-channel soft mute (`MUT12`).
    pub mute_right: Mute,
}

impl AdcControl {
    fn bits(self) -> u8 {
        (self.zero_cross.disable_bit() << 4)
            | (self.high_pass.bypass_bit() << 3)
            | (self.phase.bit() << 2)
            | (self.mute_right.bit() << 1)
            | self.mute_left.bit()
    }
}

impl Default for AdcControl {
    fn default() -> Self {
        AdcControl {
            zero_cross: ZeroCross::Enabled,
            high_pass: HighPass::Enabled,
            phase: Phase::Normal,
            mute_left: Mute::Disabled,
            mute_right: Mute::Disabled,
        }
    }
}

/// Hardware address-select pin state.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum AddressSelect {
    /// Address-select bit is low; 7-bit address is `0b100_0110`.
    Low,
    /// Address-select bit is high; 7-bit address is `0b100_0111`.
    High,
}

impl AddressSelect {
    fn bit(self) -> u8 {
        match self {
            AddressSelect::Low => 0,
            AddressSelect::High => 1,
        }
    }
}

/// Reset-bit command value.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum Reset {
    /// Write `0` to trigger the reset action.
    Trigger,
    /// Normal operation/reset inactive. This is the datasheet default.
    Normal,
}

impl Reset {
    fn bit(self) -> u8 {
        match self {
            Reset::Trigger => 0,
            Reset::Normal => 1,
        }
    }
}

/// ADC or DAC power-save state.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum Power {
    /// Release from power-save and run normally.
    Normal,
    /// Power-save mode. This is the datasheet default after power-on.
    PowerSave,
}

impl Power {
    fn power_save_bit(self) -> u8 {
        match self {
            Power::Normal => 0,
            Power::PowerSave => 1,
        }
    }
}

/// DAC analog output configuration.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum DacOutput {
    /// Differential DAC output pins.
    Differential,
    /// Single-ended DAC output pins.
    SingleEnded,
}

impl DacOutput {
    fn bit(self) -> u8 {
        match self {
            DacOutput::Differential => 0,
            DacOutput::SingleEnded => 1,
        }
    }
}

/// Clock source for DAC operation.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum DacClock {
    /// Use `SCKI2`, `BCK2`, and `LRCK2` for DAC operation.
    DacPins,
    /// Use `SCKI1`, `BCK1`, and `LRCK1` for DAC operation.
    AdcPins,
}

impl DacClock {
    fn bit(self) -> u8 {
        match self {
            DacClock::DacPins => 0,
            DacClock::AdcPins => 1,
        }
    }
}

/// Clock source for ADC operation.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum AdcClock {
    /// Use `SCKI1`, `BCK1`, and `LRCK1` for ADC operation.
    AdcPins,
    /// Use `SCKI2`, `BCK2`, and `LRCK2` for ADC operation.
    DacPins,
}

impl AdcClock {
    fn bit(self) -> u8 {
        match self {
            AdcClock::AdcPins => 0,
            AdcClock::DacPins => 1,
        }
    }
}

/// DAC audio-interface mode.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum DacMode {
    /// DAC interface is clock slave.
    Slave,
    /// DAC interface is master with 768 `fS` system-clock ratio.
    Master768Fs,
    /// DAC interface is master with 512 `fS` system-clock ratio.
    Master512Fs,
    /// DAC interface is master with 384 `fS` system-clock ratio.
    Master384Fs,
    /// DAC interface is master with 256 `fS` system-clock ratio.
    Master256Fs,
    /// DAC interface is master with 192 `fS` system-clock ratio.
    Master192Fs,
    /// DAC interface is master with 128 `fS` system-clock ratio.
    Master128Fs,
}

impl DacMode {
    fn bits(self) -> u8 {
        match self {
            DacMode::Slave => 0,
            DacMode::Master768Fs => 1,
            DacMode::Master512Fs => 2,
            DacMode::Master384Fs => 3,
            DacMode::Master256Fs => 4,
            DacMode::Master192Fs => 5,
            DacMode::Master128Fs => 6,
        }
    }

    fn master_ratio(self) -> Option<u32> {
        match self {
            DacMode::Slave => None,
            DacMode::Master768Fs => Some(768),
            DacMode::Master512Fs => Some(512),
            DacMode::Master384Fs => Some(384),
            DacMode::Master256Fs => Some(256),
            DacMode::Master192Fs => Some(192),
            DacMode::Master128Fs => Some(128),
        }
    }
}

/// ADC audio-interface mode.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum AdcMode {
    /// ADC interface is clock slave.
    Slave,
    /// ADC interface is master with 768 `fS` system-clock ratio.
    Master768Fs,
    /// ADC interface is master with 512 `fS` system-clock ratio.
    Master512Fs,
    /// ADC interface is master with 384 `fS` system-clock ratio.
    Master384Fs,
    /// ADC interface is master with 256 `fS` system-clock ratio.
    Master256Fs,
}

impl AdcMode {
    fn bits(self) -> u8 {
        match self {
            AdcMode::Slave => 0,
            AdcMode::Master768Fs => 1,
            AdcMode::Master512Fs => 2,
            AdcMode::Master384Fs => 3,
            AdcMode::Master256Fs => 4,
        }
    }

    fn master_ratio(self) -> Option<u32> {
        match self {
            AdcMode::Slave => None,
            AdcMode::Master768Fs => Some(768),
            AdcMode::Master512Fs => Some(512),
            AdcMode::Master384Fs => Some(384),
            AdcMode::Master256Fs => Some(256),
        }
    }
}

/// Serial audio data format for ADC or DAC.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum AudioFormat {
    /// 24-bit I2S format.
    I2s24,
    /// 24-bit left-justified format.
    LeftJustified24,
    /// 24-bit right-justified format.
    RightJustified24,
    /// 16-bit right-justified format.
    RightJustified16,
}

impl AudioFormat {
    fn bits(self) -> u8 {
        match self {
            AudioFormat::I2s24 => 0,
            AudioFormat::LeftJustified24 => 1,
            AudioFormat::RightJustified24 => 2,
            AudioFormat::RightJustified16 => 3,
        }
    }
}

/// DAC oversampling-rate selector.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum DacOversampling {
    /// Datasheet default oversampling family.
    Low,
    /// Higher oversampling family.
    High,
}

impl DacOversampling {
    fn bit(self) -> u8 {
        match self {
            DacOversampling::Low => 0,
            DacOversampling::High => 1,
        }
    }
}

/// Input or output phase.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum Phase {
    /// Normal polarity.
    Normal,
    /// Inverted polarity.
    Inverted,
}

impl Phase {
    fn bit(self) -> u8 {
        match self {
            Phase::Normal => 0,
            Phase::Inverted => 1,
        }
    }
}

/// Soft-mute state.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum Mute {
    /// Soft mute disabled.
    Disabled,
    /// Soft mute enabled.
    Enabled,
}

impl Mute {
    fn bit(self) -> u8 {
        match self {
            Mute::Disabled => 0,
            Mute::Enabled => 1,
        }
    }
}

/// DAC digital filter rolloff.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum FilterRolloff {
    /// Sharp rolloff.
    Sharp,
    /// Slow rolloff.
    Slow,
}

impl FilterRolloff {
    fn bit(self) -> u8 {
        match self {
            FilterRolloff::Sharp => 0,
            FilterRolloff::Slow => 1,
        }
    }
}

/// DAC de-emphasis sampling-rate selector.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum DeemphasisRate {
    /// 44.1 kHz de-emphasis curve.
    Hz44100,
    /// 48 kHz de-emphasis curve.
    Hz48000,
    /// 32 kHz de-emphasis curve.
    Hz32000,
}

impl DeemphasisRate {
    fn bits(self) -> u8 {
        match self {
            DeemphasisRate::Hz44100 => 0,
            DeemphasisRate::Hz48000 => 1,
            DeemphasisRate::Hz32000 => 2,
        }
    }
}

/// DAC de-emphasis enable.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum Deemphasis {
    /// De-emphasis disabled.
    Disabled,
    /// De-emphasis enabled.
    Enabled,
}

impl Deemphasis {
    fn bit(self) -> u8 {
        match self {
            Deemphasis::Disabled => 0,
            Deemphasis::Enabled => 1,
        }
    }
}

/// DAC zero-flag output polarity.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum ZeroFlagPolarity {
    /// Zero-detect drives the zero flag high.
    HighForZero,
    /// Zero-detect drives the zero flag low.
    LowForZero,
}

impl ZeroFlagPolarity {
    fn bit(self) -> u8 {
        match self {
            ZeroFlagPolarity::HighForZero => 0,
            ZeroFlagPolarity::LowForZero => 1,
        }
    }
}

/// DAC zero-flag pin function.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum ZeroFlagFunction {
    /// `ZEROL` and `ZEROR` report their corresponding channels independently.
    Independent,
    /// Both zero-flag pins report combined left-and-right zero detection.
    BothChannels,
}

impl ZeroFlagFunction {
    fn bit(self) -> u8 {
        match self {
            ZeroFlagFunction::Independent => 0,
            ZeroFlagFunction::BothChannels => 1,
        }
    }
}

/// ADC zero-cross detection behavior for attenuation and mute changes.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum ZeroCross {
    /// Change attenuation/mute at zero-crossing, with datasheet timeout behavior.
    Enabled,
    /// Disable zero-cross timing; changes occur at the 8/`fS` interval.
    Disabled,
}

impl ZeroCross {
    fn disable_bit(self) -> u8 {
        match self {
            ZeroCross::Enabled => 0,
            ZeroCross::Disabled => 1,
        }
    }
}

/// ADC high-pass filter state.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum HighPass {
    /// High-pass filter enabled. This is the datasheet default.
    Enabled,
    /// High-pass filter bypassed, preserving DC.
    Bypassed,
}

impl HighPass {
    fn bypass_bit(self) -> u8 {
        match self {
            HighPass::Enabled => 0,
            HighPass::Bypassed => 1,
        }
    }
}

/// DAC digital attenuation register value.
///
/// The datasheet mapping is `0.5 * (value - 255)` dB for values `55..=255`.
/// Values `0..=54` mute the DAC channel.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct DacAttenuation(
    /// Raw DAC attenuation register value.
    pub u8,
);

impl DacAttenuation {
    /// 0 dB DAC attenuation.
    pub const ZERO_DB: Self = DacAttenuation(255);
    /// Infinite DAC attenuation/mute.
    pub const MUTE: Self = DacAttenuation(0);

    /// Create DAC attenuation from 0.5 dB steps below 0 dB.
    ///
    /// `0` means 0 dB, `1` means -0.5 dB, and `200` means -100 dB.
    /// Larger values map to mute.
    pub fn from_half_db_steps_below_zero(steps: u8) -> Self {
        if steps > 200 {
            DacAttenuation::MUTE
        } else {
            DacAttenuation(255 - steps)
        }
    }
}

/// ADC digital attenuation/gain register value.
///
/// The datasheet mapping is `0.5 * (value - 215)` dB for values `15..=255`.
/// Values `0..=14` mute the ADC channel.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct AdcAttenuation(
    /// Raw ADC attenuation register value.
    pub u8,
);

impl AdcAttenuation {
    /// +20 dB ADC gain.
    pub const PLUS_20_DB: Self = AdcAttenuation(255);
    /// 0 dB ADC gain/attenuation.
    pub const ZERO_DB: Self = AdcAttenuation(215);
    /// Infinite ADC attenuation/mute.
    pub const MUTE: Self = AdcAttenuation(0);

    /// Create ADC attenuation/gain from 0.5 dB steps relative to 0 dB.
    ///
    /// `0` means 0 dB, `1` means +0.5 dB, `-1` means -0.5 dB.
    /// Values above +20 dB clamp to +20 dB; values at or below the mute
    /// threshold map to mute.
    pub fn from_half_db_steps_relative_to_zero(steps: i16) -> Self {
        let value = 215 + steps;
        if value <= 14 {
            AdcAttenuation::MUTE
        } else if value >= 255 {
            AdcAttenuation::PLUS_20_DB
        } else {
            AdcAttenuation(value as u8)
        }
    }
}

/// Error returned by an I2C control-port write.
#[derive(Debug, defmt::Format, PartialEq, Eq)]
pub struct Error<E> {
    /// The typed control operation being written when the I2C error occurred.
    pub operation: Operation,
    /// The underlying I2C error.
    #[defmt(Debug2Format)]
    pub source: E,
}

/// Error returned by a validated configuration write.
#[derive(Debug, defmt::Format, PartialEq, Eq)]
pub enum ConfigureError<E> {
    /// The requested configuration was invalid.
    InvalidConfig(ConfigError),
    /// A control-port write failed.
    Write(Error<E>),
}

/// System-clock/sample-rate ratio error.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum ClockError {
    /// The requested audio sample rate was zero.
    ZeroSampleRate,
    /// The system clock is not an integer multiple of the sample rate.
    NonIntegerRatio {
        /// System clock input frequency in hertz.
        scki_hz: u32,
        /// Audio sample rate in hertz.
        fs_hz: u32,
    },
    /// The ADC does not support this `SCKI1/fS` ratio.
    UnsupportedAdcRatio(u32),
    /// The DAC does not support this `SCKI2/fS` ratio.
    UnsupportedDacRatio(u32),
}

/// Supported ADC system-clock ratio.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum AdcClockRatio {
    /// `SCKI1 = 256 * fS`.
    Fs256,
    /// `SCKI1 = 384 * fS`.
    Fs384,
    /// `SCKI1 = 512 * fS`.
    Fs512,
    /// `SCKI1 = 768 * fS`.
    Fs768,
}

/// Supported DAC system-clock ratio.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum DacClockRatio {
    /// `SCKI2 = 128 * fS`.
    Fs128,
    /// `SCKI2 = 192 * fS`.
    Fs192,
    /// `SCKI2 = 256 * fS`.
    Fs256,
    /// `SCKI2 = 384 * fS`.
    Fs384,
    /// `SCKI2 = 512 * fS`.
    Fs512,
    /// `SCKI2 = 768 * fS`.
    Fs768,
}

/// Validate an ADC system-clock input against a sample rate.
///
/// The PCM3060 ADC accepts integer `SCKI1/fS` ratios of 256, 384, 512, or 768.
pub fn validate_adc_clock(scki_hz: u32, fs_hz: u32) -> Result<AdcClockRatio, ClockError> {
    let ratio = clock_ratio(scki_hz, fs_hz)?;
    match ratio {
        256 => Ok(AdcClockRatio::Fs256),
        384 => Ok(AdcClockRatio::Fs384),
        512 => Ok(AdcClockRatio::Fs512),
        768 => Ok(AdcClockRatio::Fs768),
        ratio => Err(ClockError::UnsupportedAdcRatio(ratio)),
    }
}

impl AdcClockRatio {
    fn ratio(self) -> u32 {
        match self {
            AdcClockRatio::Fs256 => 256,
            AdcClockRatio::Fs384 => 384,
            AdcClockRatio::Fs512 => 512,
            AdcClockRatio::Fs768 => 768,
        }
    }
}

/// Validate a DAC system-clock input against a sample rate.
///
/// The PCM3060 DAC accepts integer `SCKI2/fS` ratios of 128, 192, 256, 384,
/// 512, or 768.
pub fn validate_dac_clock(scki_hz: u32, fs_hz: u32) -> Result<DacClockRatio, ClockError> {
    let ratio = clock_ratio(scki_hz, fs_hz)?;
    match ratio {
        128 => Ok(DacClockRatio::Fs128),
        192 => Ok(DacClockRatio::Fs192),
        256 => Ok(DacClockRatio::Fs256),
        384 => Ok(DacClockRatio::Fs384),
        512 => Ok(DacClockRatio::Fs512),
        768 => Ok(DacClockRatio::Fs768),
        ratio => Err(ClockError::UnsupportedDacRatio(ratio)),
    }
}

impl DacClockRatio {
    fn ratio(self) -> u32 {
        match self {
            DacClockRatio::Fs128 => 128,
            DacClockRatio::Fs192 => 192,
            DacClockRatio::Fs256 => 256,
            DacClockRatio::Fs384 => 384,
            DacClockRatio::Fs512 => 512,
            DacClockRatio::Fs768 => 768,
        }
    }
}

fn clock_ratio(scki_hz: u32, fs_hz: u32) -> Result<u32, ClockError> {
    if fs_hz == 0 {
        return Err(ClockError::ZeroSampleRate);
    }
    if scki_hz % fs_hz != 0 {
        return Err(ClockError::NonIntegerRatio { scki_hz, fs_hz });
    }
    Ok(scki_hz / fs_hz)
}

/// Calculate the converter power-save release wait in microseconds.
///
/// The PCM3060 datasheet specifies a `2048 / fS` wait after changing ADC or
/// DAC power-save state. This returns that interval rounded up to whole
/// microseconds, or `0` for `fs_hz == 0`.
pub fn power_change_delay_us(fs_hz: u32) -> u32 {
    if fs_hz == 0 {
        return 0;
    }
    (((2048_u64 * 1_000_000) + fs_hz as u64 - 1) / fs_hz as u64) as u32
}

fn min_nonzero(a: u32, b: u32) -> u32 {
    match (a, b) {
        (0, b) => b,
        (a, 0) => a,
        (a, b) => a.min(b),
    }
}

fn validate_no_reset_commands(config: &Config) -> Result<(), ConfigError> {
    if config.control.mode_control_reset == Reset::Trigger
        || config.control.system_reset == Reset::Trigger
    {
        Err(ConfigError::ResetCommandInConfig)
    } else {
        Ok(())
    }
}

fn validate_power_release_sample_rates(
    config: &Config,
    adc_fs_hz: u32,
    dac_fs_hz: u32,
) -> Result<(), ConfigError> {
    if config.control.adc_power == Power::Normal && adc_fs_hz == 0 {
        return Err(ConfigError::AdcClock(ClockError::ZeroSampleRate));
    }
    if config.control.dac_power == Power::Normal && dac_fs_hz == 0 {
        return Err(ConfigError::DacClock(ClockError::ZeroSampleRate));
    }
    Ok(())
}

fn validate_clocks(config: &Config, clocks: ClockConfig) -> Result<(), ConfigError> {
    if let (Some(scki_hz), Some(fs_hz)) = (clocks.adc_scki_hz, clocks.adc_fs_hz) {
        let ratio = validate_adc_clock(scki_hz, fs_hz).map_err(ConfigError::AdcClock)?;
        validate_adc_master_ratio(config, ratio)?;
    }

    if let (Some(scki_hz), Some(fs_hz)) = (clocks.dac_scki_hz, clocks.dac_fs_hz) {
        let ratio = validate_dac_clock(scki_hz, fs_hz).map_err(ConfigError::DacClock)?;
        validate_dac_master_ratio(config, ratio)?;
    }

    Ok(())
}

fn validate_adc_master_ratio(config: &Config, ratio: AdcClockRatio) -> Result<(), ConfigError> {
    if let Some(mode_ratio) = config.adc.interface.mode.master_ratio() {
        let clock_ratio = ratio.ratio();
        if mode_ratio != clock_ratio {
            return Err(ConfigError::AdcMasterRatioMismatch {
                mode_ratio,
                clock_ratio,
            });
        }
    }
    Ok(())
}

fn validate_dac_master_ratio(config: &Config, ratio: DacClockRatio) -> Result<(), ConfigError> {
    if let Some(mode_ratio) = config.dac.interface.mode.master_ratio() {
        let clock_ratio = ratio.ratio();
        if mode_ratio != clock_ratio {
            return Err(ConfigError::DacMasterRatioMismatch {
                mode_ratio,
                clock_ratio,
            });
        }
    }
    Ok(())
}

/// Error returned by an external PCM3060 control pin.
#[derive(Debug, defmt::Format, PartialEq, Eq)]
pub enum PinError<ResetError, PowerDownError> {
    /// The external `RST` pin operation failed.
    Reset(#[defmt(Debug2Format)] ResetError),
    /// The external `PDWN` pin operation failed.
    PowerDown(#[defmt(Debug2Format)] PowerDownError),
}

/// External PCM3060 `RST` and `PDWN` control pins.
///
/// Both pins are active low. Driving `RST` low asserts hardware reset; driving
/// `PDWN` low enters hardware power-down.
pub struct ControlPins<RST, PDWN> {
    reset: RST,
    power_down: PDWN,
}

impl<RST, PDWN> ControlPins<RST, PDWN> {
    /// Create a helper around caller-owned active-low `RST` and `PDWN` pins.
    pub fn new(reset: RST, power_down: PDWN) -> Self {
        ControlPins { reset, power_down }
    }

    /// Return the owned pins.
    pub fn release(self) -> (RST, PDWN) {
        (self.reset, self.power_down)
    }
}

impl<RST, PDWN, ResetError, PowerDownError> ControlPins<RST, PDWN>
where
    RST: OutputPin<Error = ResetError>,
    PDWN: OutputPin<Error = PowerDownError>,
{
    /// Assert the active-low hardware reset pin.
    pub fn assert_reset(&mut self) -> Result<(), PinError<ResetError, PowerDownError>> {
        self.reset.set_low().map_err(PinError::Reset)
    }

    /// Release the active-low hardware reset pin.
    pub fn release_reset(&mut self) -> Result<(), PinError<ResetError, PowerDownError>> {
        self.reset.set_high().map_err(PinError::Reset)
    }

    /// Drive the active-low hardware power-down pin into power-down.
    pub fn power_down(&mut self) -> Result<(), PinError<ResetError, PowerDownError>> {
        self.power_down.set_low().map_err(PinError::PowerDown)
    }

    /// Release the active-low hardware power-down pin.
    pub fn power_up(&mut self) -> Result<(), PinError<ResetError, PowerDownError>> {
        self.power_down.set_high().map_err(PinError::PowerDown)
    }

    /// Pulse hardware reset low, wait `low_us`, then release reset.
    pub fn reset_pulse<D>(
        &mut self,
        low_us: u32,
        delay: &mut D,
    ) -> Result<(), PinError<ResetError, PowerDownError>>
    where
        D: DelayUs<u32>,
    {
        self.assert_reset()?;
        delay.delay_us(low_us);
        self.release_reset()
    }
}

/// Error returned by DAC zero-flag input pins.
#[derive(Debug, defmt::Format, PartialEq, Eq)]
pub enum ZeroFlagError<LeftError, RightError> {
    /// Reading `ZEROL` failed.
    Left(#[defmt(Debug2Format)] LeftError),
    /// Reading `ZEROR` failed.
    Right(#[defmt(Debug2Format)] RightError),
}

/// External PCM3060 DAC zero-detect flag pins.
///
/// Pass the same [`ZeroFlagPolarity`] that was written in [`DacFilter`] so the
/// returned booleans are logical zero-detect states instead of raw pin levels.
pub struct ZeroFlagPins<L, R> {
    left: L,
    right: R,
}

impl<L, R> ZeroFlagPins<L, R> {
    /// Create a helper around caller-owned `ZEROL` and `ZEROR` pins.
    pub fn new(left: L, right: R) -> Self {
        ZeroFlagPins { left, right }
    }

    /// Return the owned pins.
    pub fn release(self) -> (L, R) {
        (self.left, self.right)
    }
}

impl<L, R, LeftError, RightError> ZeroFlagPins<L, R>
where
    L: InputPin<Error = LeftError>,
    R: InputPin<Error = RightError>,
{
    /// Return whether `ZEROL` indicates a zero-detect condition.
    pub fn left_zero(
        &self,
        polarity: ZeroFlagPolarity,
    ) -> Result<bool, ZeroFlagError<LeftError, RightError>> {
        pin_zero(&self.left, polarity).map_err(ZeroFlagError::Left)
    }

    /// Return whether `ZEROR` indicates a zero-detect condition.
    pub fn right_zero(
        &self,
        polarity: ZeroFlagPolarity,
    ) -> Result<bool, ZeroFlagError<LeftError, RightError>> {
        pin_zero(&self.right, polarity).map_err(ZeroFlagError::Right)
    }

    /// Return whether both zero-flag pins indicate zero-detect conditions.
    pub fn both_zero(
        &self,
        polarity: ZeroFlagPolarity,
    ) -> Result<bool, ZeroFlagError<LeftError, RightError>> {
        Ok(self.left_zero(polarity)? && self.right_zero(polarity)?)
    }
}

fn pin_zero<P, E>(pin: &P, polarity: ZeroFlagPolarity) -> Result<bool, E>
where
    P: InputPin<Error = E>,
{
    match polarity {
        ZeroFlagPolarity::HighForZero => pin.is_high(),
        ZeroFlagPolarity::LowForZero => pin.is_low(),
    }
}

/// One stereo frame represented as normalized floating-point samples.
///
/// Samples are in the conventional `[-1.0, 1.0]` range. Conversion to the
/// PCM3060 24-bit transport format clamps to that range.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq)]
pub struct StereoF32 {
    /// Left channel sample.
    pub left: f32,
    /// Right channel sample.
    pub right: f32,
}

/// Error returned when creating a zero-copy PCM buffer view.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub enum BufferError {
    /// The slice length is not an exact number of stereo frames for the format.
    IncompleteFrame {
        /// Number of `u16` words in each stereo frame for the selected format.
        words_per_frame: usize,
        /// Number of trailing words that do not form a complete frame.
        trailing_words: usize,
    },
}

/// Marker trait for PCM3060 DMA/sample buffer formats.
pub trait PcmFormat {
    /// Matching PCM3060 serial audio format.
    const AUDIO_FORMAT: AudioFormat;
    /// Number of `u16` words in one stereo frame.
    const WORDS_PER_FRAME: usize;

    #[doc(hidden)]
    fn read_frame(words: &[u16]) -> StereoF32;
    #[doc(hidden)]
    fn write_frame(frame: StereoF32, words: &mut [u16]);
}

/// 24-bit I2S samples held in 32-bit slots.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct I2s24;

/// 24-bit left-justified samples held in 32-bit slots.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct LeftJustified24;

/// 24-bit right-justified samples held in 32-bit slots.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct RightJustified24;

/// 16-bit right-justified samples held as one `u16` word per channel.
#[derive(Clone, Copy, Debug, defmt::Format, PartialEq, Eq)]
pub struct RightJustified16;

macro_rules! impl_pcm24_format {
    ($format:ty, $audio_format:expr) => {
        impl PcmFormat for $format {
            const AUDIO_FORMAT: AudioFormat = $audio_format;
            const WORDS_PER_FRAME: usize = 4;

            fn read_frame(words: &[u16]) -> StereoF32 {
                StereoF32 {
                    left: pcm24_words_to_f32(words[0], words[1]),
                    right: pcm24_words_to_f32(words[2], words[3]),
                }
            }

            fn write_frame(frame: StereoF32, words: &mut [u16]) {
                let left = f32_to_pcm24_words(frame.left);
                let right = f32_to_pcm24_words(frame.right);
                words[0] = left[0];
                words[1] = left[1];
                words[2] = right[0];
                words[3] = right[1];
            }
        }
    };
}

impl_pcm24_format!(I2s24, AudioFormat::I2s24);
impl_pcm24_format!(LeftJustified24, AudioFormat::LeftJustified24);
impl_pcm24_format!(RightJustified24, AudioFormat::RightJustified24);

impl PcmFormat for RightJustified16 {
    const AUDIO_FORMAT: AudioFormat = AudioFormat::RightJustified16;
    const WORDS_PER_FRAME: usize = 2;

    fn read_frame(words: &[u16]) -> StereoF32 {
        StereoF32 {
            left: pcm16_word_to_f32(words[0]),
            right: pcm16_word_to_f32(words[1]),
        }
    }

    fn write_frame(frame: StereoF32, words: &mut [u16]) {
        words[0] = f32_to_pcm16_word(frame.left);
        words[1] = f32_to_pcm16_word(frame.right);
    }
}

/// Zero-copy read view over a PCM3060 stereo input buffer.
pub struct PcmIn<'a, F> {
    words: &'a [u16],
    _format: PhantomData<F>,
}

impl<'a, F> PcmIn<'a, F>
where
    F: PcmFormat,
{
    /// Create an input view over caller-owned DMA/sample memory.
    ///
    /// Returns an error if `words` is not an exact number of stereo frames for
    /// the selected format.
    pub fn new(words: &'a [u16]) -> Result<Self, BufferError> {
        validate_buffer_words::<F>(words.len())?;
        Ok(PcmIn {
            words,
            _format: PhantomData,
        })
    }

    /// Create an input view without validating frame alignment.
    pub fn new_unchecked(words: &'a [u16]) -> Self {
        PcmIn {
            words,
            _format: PhantomData,
        }
    }

    /// Iterate over complete stereo frames without copying the backing buffer.
    pub fn frames(&self) -> impl Iterator<Item = StereoF32> + '_ {
        self.words
            .chunks_exact(F::WORDS_PER_FRAME)
            .map(F::read_frame)
    }
}

/// Zero-copy write view over a PCM3060 stereo output buffer.
pub struct PcmOut<'a, F> {
    words: &'a mut [u16],
    _format: PhantomData<F>,
}

impl<'a, F> PcmOut<'a, F>
where
    F: PcmFormat,
{
    /// Create an output view over caller-owned DMA/sample memory.
    ///
    /// Returns an error if `words` is not an exact number of stereo frames for
    /// the selected format.
    pub fn new(words: &'a mut [u16]) -> Result<Self, BufferError> {
        validate_buffer_words::<F>(words.len())?;
        Ok(PcmOut {
            words,
            _format: PhantomData,
        })
    }

    /// Create an output view without validating frame alignment.
    pub fn new_unchecked(words: &'a mut [u16]) -> Self {
        PcmOut {
            words,
            _format: PhantomData,
        }
    }

    /// Write stereo frames into the backing buffer without allocating or copying blocks.
    ///
    /// Writing stops when either the frame iterator is exhausted or the output
    /// buffer has no remaining complete stereo frame.
    pub fn write_frames<I>(&mut self, frames: I)
    where
        I: IntoIterator<Item = StereoF32>,
    {
        for (output, frame) in self.words.chunks_exact_mut(F::WORDS_PER_FRAME).zip(frames) {
            F::write_frame(frame, output);
        }
    }
}

/// Zero-copy read view over 24-bit I2S stereo input in 32-bit slots.
pub type Pcm24In<'a> = PcmIn<'a, I2s24>;

/// Zero-copy write view over 24-bit I2S stereo output in 32-bit slots.
pub type Pcm24Out<'a> = PcmOut<'a, I2s24>;

/// Zero-copy read view over 16-bit right-justified stereo input.
pub type Pcm16In<'a> = PcmIn<'a, RightJustified16>;

/// Zero-copy write view over 16-bit right-justified stereo output.
pub type Pcm16Out<'a> = PcmOut<'a, RightJustified16>;

fn validate_buffer_words<F>(len: usize) -> Result<(), BufferError>
where
    F: PcmFormat,
{
    let trailing_words = len % F::WORDS_PER_FRAME;
    if trailing_words == 0 {
        Ok(())
    } else {
        Err(BufferError::IncompleteFrame {
            words_per_frame: F::WORDS_PER_FRAME,
            trailing_words,
        })
    }
}

/// Convert one 24-bit two's-complement sample held in two `u16` words to `f32`.
///
/// `hi` is the upper 16 bits of the 32-bit I2S slot; `lo` is the lower 16 bits.
pub fn pcm24_words_to_f32(hi: u16, lo: u16) -> f32 {
    let hi = (hi as i32) << 16;
    let lo = lo as i32;
    let n = hi | lo;
    n as f32 / i32::MAX as f32
}

/// Convert a normalized `f32` sample into one 24-bit two's-complement sample.
///
/// The sample is clamped to `[-1.0, 1.0]` before conversion.
pub fn f32_to_pcm24_words(sample: f32) -> [u16; 2] {
    let n = (sample.clamp(-1.0, 1.0) * i32::MAX as f32) as i32;
    [(n >> 16) as u16, n as u16]
}

/// Convert one 16-bit two's-complement sample to `f32`.
pub fn pcm16_word_to_f32(word: u16) -> f32 {
    (word as i16) as f32 / i16::MAX as f32
}

/// Convert a normalized `f32` sample into one 16-bit two's-complement word.
///
/// The sample is clamped to `[-1.0, 1.0]` before conversion.
pub fn f32_to_pcm16_word(sample: f32) -> u16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16 as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct I2cLog {
        writes: heapless::Vec<(u8, heapless::Vec<u8, 2>), 16>,
    }

    impl I2cLog {
        fn new() -> Self {
            I2cLog {
                writes: heapless::Vec::new(),
            }
        }
    }

    impl WriteIter for I2cLog {
        type Error = ();

        fn write<B>(&mut self, addr: u8, bytes: B) -> Result<(), Self::Error>
        where
            B: IntoIterator<Item = u8>,
        {
            let mut write = heapless::Vec::new();
            for byte in bytes {
                write.push(byte).unwrap();
            }
            self.writes.push((addr, write)).unwrap();
            Ok(())
        }
    }

    #[derive(Default)]
    struct DelayLog {
        us: heapless::Vec<u32, 4>,
    }

    impl DelayUs<u32> for DelayLog {
        fn delay_us(&mut self, us: u32) {
            self.us.push(us).unwrap();
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct OutputLog {
        high: bool,
    }

    impl OutputLog {
        fn low() -> Self {
            OutputLog { high: false }
        }
    }

    impl OutputPin for OutputLog {
        type Error = ();

        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.high = false;
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.high = true;
            Ok(())
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct InputLog {
        high: bool,
    }

    impl InputPin for InputLog {
        type Error = ();

        fn is_high(&self) -> Result<bool, Self::Error> {
            Ok(self.high)
        }

        fn is_low(&self) -> Result<bool, Self::Error> {
            Ok(!self.high)
        }
    }

    #[test]
    fn default_config_matches_reset_register_values() {
        let mut codec = Pcm3060::new(I2cLog::new(), AddressSelect::Low);

        codec.configure(&Config::default()).unwrap();

        let i2c = codec.release();
        let writes = i2c.writes;
        assert_eq!(writes[0].1.as_slice(), &[0x40, 0b1111_0000]);
        assert_eq!(writes[1].1.as_slice(), &[0x41, 0xff]);
        assert_eq!(writes[2].1.as_slice(), &[0x42, 0xff]);
        assert_eq!(writes[3].1.as_slice(), &[0x43, 0x00]);
        assert_eq!(writes[4].1.as_slice(), &[0x44, 0x00]);
        assert_eq!(writes[5].1.as_slice(), &[0x45, 0x00]);
        assert_eq!(writes[6].1.as_slice(), &[0x46, 0xd7]);
        assert_eq!(writes[7].1.as_slice(), &[0x47, 0xd7]);
        assert_eq!(writes[8].1.as_slice(), &[0x48, 0x00]);
        assert_eq!(writes[9].1.as_slice(), &[0x49, 0x00]);
    }

    #[test]
    fn bringup_config_writes_single_ended_hpf_bypass_then_normal_power() {
        let mut codec = Pcm3060::new(I2cLog::new(), AddressSelect::Low);

        codec.configure_single_ended_no_adc_high_pass().unwrap();

        let i2c = codec.release();
        assert_eq!(i2c.writes[0].1.as_slice(), &[0x40, 0b1111_0001]);
        assert_eq!(i2c.writes[1].1.as_slice(), &[0x49, 0b0000_1000]);
        assert_eq!(i2c.writes[2].1.as_slice(), &[0x40, 0b1100_0001]);
    }

    #[test]
    fn interface_bits_encode_all_fields() {
        let dac = DacInterface {
            clock: DacClock::AdcPins,
            mode: DacMode::Master192Fs,
            format: AudioFormat::RightJustified16,
        };
        let adc = AdcInterface {
            clock: AdcClock::DacPins,
            mode: AdcMode::Master256Fs,
            format: AudioFormat::LeftJustified24,
        };

        assert_eq!(dac.bits(), 0b1101_0011);
        assert_eq!(adc.bits(), 0b1100_0001);
    }

    #[test]
    fn high_address_select_sets_low_address_bit() {
        let mut codec = Pcm3060::new(I2cLog::new(), AddressSelect::High);

        codec.write_control(Control::default()).unwrap();

        let i2c = codec.release();
        assert_eq!(i2c.writes[0].0, BASE_ADDR | 1);
    }

    #[test]
    fn configure_with_power_release_delay_powers_up_after_typed_config() {
        let mut codec = Pcm3060::new(I2cLog::new(), AddressSelect::Low);
        let mut delay = DelayLog::default();
        let mut config = Config::default();
        config.control.adc_power = Power::Normal;
        config.control.dac_power = Power::Normal;

        codec
            .configure_with_power_release_delay(&config, 48_000, 96_000, &mut delay)
            .unwrap();

        let i2c = codec.release();
        assert_eq!(i2c.writes[0].1.as_slice(), &[0x40, 0b1111_0000]);
        assert_eq!(i2c.writes[10].1.as_slice(), &[0x40, 0b1100_0000]);
        assert_eq!(delay.us.as_slice(), &[42_667]);
    }

    #[test]
    fn configure_with_power_release_delay_rejects_missing_sample_rate() {
        let mut codec = Pcm3060::new(I2cLog::new(), AddressSelect::Low);
        let mut delay = DelayLog::default();
        let mut config = Config::default();
        config.control.adc_power = Power::Normal;

        let result = codec.configure_with_power_release_delay(&config, 0, 48_000, &mut delay);

        assert_eq!(
            result,
            Err(ConfigureError::InvalidConfig(ConfigError::AdcClock(
                ClockError::ZeroSampleRate
            )))
        );
    }

    #[test]
    fn phase_builder_rejects_reset_commands_and_master_ratio_mismatches() {
        let mut reset_config = Config::default();
        reset_config.control.system_reset = Reset::Trigger;
        assert_eq!(
            ConfigBuilder::from_config(reset_config),
            Err(ConfigError::ResetCommandInConfig)
        );

        assert_eq!(
            Config::builder()
                .adc_mode(AdcMode::Master384Fs)
                .clocks(48_000, 12_288_000, 12_288_000),
            Err(ConfigError::AdcMasterRatioMismatch {
                mode_ratio: 384,
                clock_ratio: 256
            })
        );
    }

    #[test]
    fn phase_builder_builds_ready_validated_config_infallibly() {
        let setup = Config::builder()
            .dac_output(DacOutput::SingleEnded)
            .adc_high_pass(HighPass::Bypassed)
            .audio_format(AudioFormat::I2s24)
            .clocks(48_000, 12_288_000, 12_288_000)
            .unwrap()
            .powered_up()
            .build();

        let config = setup.config();

        assert_eq!(config.control.adc_power, Power::Normal);
        assert_eq!(config.control.dac_power, Power::Normal);
        assert_eq!(config.control.dac_output, DacOutput::SingleEnded);
        assert_eq!(config.adc.control.high_pass, HighPass::Bypassed);
        assert_eq!(setup.clocks().unwrap().adc_ratio, AdcClockRatio::Fs256);
    }

    #[test]
    fn configure_validated_uses_setup_clocks_for_power_release_delay() {
        let setup = Config::builder()
            .clocks(48_000, 12_288_000, 12_288_000)
            .unwrap()
            .powered_up()
            .build();
        let mut codec = Pcm3060::new(I2cLog::new(), AddressSelect::Low);
        let mut delay = DelayLog::default();

        codec.configure_validated(&setup, &mut delay).unwrap();

        let i2c = codec.release();
        assert_eq!(i2c.writes[0].1.as_slice(), &[0x40, 0b1111_0000]);
        assert_eq!(i2c.writes[10].1.as_slice(), &[0x40, 0b1100_0000]);
        assert_eq!(delay.us.as_slice(), &[42_667]);
    }

    #[test]
    fn clock_validation_accepts_only_pcm3060_ratios() {
        assert_eq!(
            validate_adc_clock(12_288_000, 48_000),
            Ok(AdcClockRatio::Fs256)
        );
        assert_eq!(
            validate_dac_clock(6_144_000, 48_000),
            Ok(DacClockRatio::Fs128)
        );
        assert_eq!(
            validate_adc_clock(18_432_000, 48_000),
            Ok(AdcClockRatio::Fs384)
        );
        assert_eq!(
            validate_dac_clock(12_300_000, 48_000),
            Err(ClockError::NonIntegerRatio {
                scki_hz: 12_300_000,
                fs_hz: 48_000
            })
        );
        assert_eq!(
            validate_adc_clock(6_144_000, 48_000),
            Err(ClockError::UnsupportedAdcRatio(128))
        );
    }

    #[test]
    fn control_pins_drive_active_low_reset_and_power_down() {
        let reset = OutputLog::low();
        let power_down = OutputLog::low();
        let mut pins = ControlPins::new(reset, power_down);
        let mut delay = DelayLog::default();

        pins.power_up().unwrap();
        pins.reset_pulse(10, &mut delay).unwrap();

        let (reset, power_down) = pins.release();
        assert!(reset.high);
        assert!(power_down.high);
        assert_eq!(delay.us.as_slice(), &[10]);
    }

    #[test]
    fn zero_flag_pins_apply_configured_polarity() {
        let pins = ZeroFlagPins::new(InputLog { high: true }, InputLog { high: false });

        assert!(pins.left_zero(ZeroFlagPolarity::HighForZero).unwrap());
        assert!(!pins.right_zero(ZeroFlagPolarity::HighForZero).unwrap());
        assert!(!pins.left_zero(ZeroFlagPolarity::LowForZero).unwrap());
        assert!(pins.right_zero(ZeroFlagPolarity::LowForZero).unwrap());
        assert!(!pins.both_zero(ZeroFlagPolarity::HighForZero).unwrap());
    }

    #[test]
    fn pcm24_views_read_and_write_stereo_frames_without_copying_buffers() {
        let input = [0x7fff, 0xffff, 0x8000, 0x0000];
        let mut output = [0_u16; 4];
        let input = Pcm24In::new(&input).unwrap();
        let frames = input.frames().map(|mut frame| {
            frame.left *= 0.5;
            frame.right *= 0.5;
            frame
        });

        Pcm24Out::new(&mut output).unwrap().write_frames(frames);

        assert_eq!(output, [0x4000, 0x0000, 0xc000, 0x0000]);
    }

    #[test]
    fn pcm16_views_read_and_write_stereo_frames_without_copying_buffers() {
        let input = [0x7fff, 0x8000];
        let mut output = [0_u16; 2];
        let input = Pcm16In::new(&input).unwrap();
        let frames = input.frames().map(|mut frame| {
            frame.left *= 0.5;
            frame.right *= 0.5;
            frame
        });

        Pcm16Out::new(&mut output).unwrap().write_frames(frames);

        assert_eq!(output, [0x3fff, 0xc000]);
    }

    #[test]
    fn pcm_views_reject_incomplete_frames() {
        let input = [0_u16; 3];
        assert!(matches!(
            Pcm24In::new(&input),
            Err(BufferError::IncompleteFrame {
                words_per_frame: 4,
                trailing_words: 3
            })
        ));

        let mut output = [0_u16; 3];
        assert!(matches!(
            Pcm16Out::new(&mut output),
            Err(BufferError::IncompleteFrame {
                words_per_frame: 2,
                trailing_words: 1
            })
        ));
    }
}
