//! Interface around Apple's SMC API.
//!
//! # References
//! * <https://github.com/shurizzle/rust-smc>
//! * <https://github.com/exelban/stats/blob/master/SMC/smc.swift>

#![allow(
    clippy::ptr_as_ptr,
    clippy::borrow_as_ptr,
    clippy::cast_ptr_alignment,
    clippy::mutex_atomic
)]

mod conv;
mod sys;

use self::{conv::*, sys::*};
use std::{
    collections::HashMap,
    fmt,
    os::raw::c_void,
    sync::{Arc, Mutex},
};

use four_char_code::{four_char_code, FourCharCode};
use libc::{sysctl, CTL_HW};

#[derive(Default, Debug, Copy, Clone)]
pub struct SmcBytes(pub(crate) [u8; 32]);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct DataType {
    pub id: FourCharCode,
    pub size: u32,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
pub struct SmcKey {
    pub code: FourCharCode,
    pub info: DataType,
}

macro_rules! fcc_format {
    ($fmt:literal, $( $args:expr ),+) => {
        Into::<FourCharCode>::into(format!($fmt, $($args),+))
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
enum SmcSelector {
    Unknown = 0,
    ReadKey = 5,
    WriteKey = 6,
    GetKeyFromIndex = 8,
    GetKeyInfo = 9,
}

impl Default for SmcSelector {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SmcVersion {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SmcPLimitData {
    version: u16,
    length: u16,
    cpu_plimit: u32,
    gpu_plimit: u32,
    mem_plimit: u32,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SmcKeyInfoData {
    data_size: u32,
    data_type: FourCharCode,
    data_attributes: u8,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SmcParam {
    key: FourCharCode,
    vers: SmcVersion,
    p_limit_data: SmcPLimitData,
    key_info: SmcKeyInfoData,
    result: u8,
    status: u8,
    selector: SmcSelector,
    data32: u32,
    bytes: SmcBytes,
}

macro_rules! err_system {
    ($err:literal) => {
        (($err & 0x3f) << 26)
    };
}

macro_rules! err_sub {
    ($err:literal) => {
        (($err & 0xfff) << 14)
    };
}

const SYS_IOKIT: kern_return_t = err_system!(0x38);
const SUB_IOKIT_COMMON: kern_return_t = err_sub!(0);

macro_rules! iokit_common_err {
    ($err:literal) => {
        SYS_IOKIT | SUB_IOKIT_COMMON | $err
    };
}

const KERN_SUCCESS: kern_return_t = 0;
#[allow(non_upper_case_globals)]
const kIOReturnSuccess: kern_return_t = KERN_SUCCESS;
#[allow(non_upper_case_globals)]
const kIOReturnNotPrivileged: kern_return_t = iokit_common_err!(0x2c1);

const MACH_PORT_NULL: mach_port_t = 0 as mach_port_t;
#[allow(non_upper_case_globals)]
const kIOMasterPortDefault: mach_port_t = MACH_PORT_NULL;

const TYPE_SP78: FourCharCode = four_char_code!("sp78");
const TYPE_FLT: FourCharCode = four_char_code!("flt ");

const HW_PACKAGES: i32 = 125;
const HW_PHYSICALCPU: i32 = 101;

#[derive(Debug)]
pub enum SmcError {
    DriverNotFound,
    FailedToOpen,
    KeyNotFound(FourCharCode),
    NotPrivileged,
    UnsafeFanSpeed,
    Unknown(i32, u8),
    Sysctl(i32),
}

impl SmcError {
    pub const fn code(&self) -> Option<FourCharCode> {
        match self {
            Self::KeyNotFound(code) => Some(*code),
            _ => None,
        }
    }

    pub const fn io_result(&self) -> Option<i32> {
        match self {
            Self::Unknown(io_res, _) => Some(*io_res),
            _ => None,
        }
    }

    pub const fn smc_result(&self) -> Option<u8> {
        match self {
            Self::Unknown(_, smc_res) => Some(*smc_res),
            _ => None,
        }
    }
}

impl fmt::Display for SmcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DriverNotFound => write!(f, "Driver not found"),
            Self::FailedToOpen => write!(f, "Failed to open driver"),
            Self::KeyNotFound(code) => write!(f, "SMC Key {:?} not found", code),
            Self::NotPrivileged => write!(
                f,
                "Not enough privileges to perform this action, are you running on root?"
            ),
            Self::UnsafeFanSpeed => write!(f, "Unsafe fan speed"),
            Self::Unknown(io_res, smc_res) => write!(
                f,
                "Unknown error: IOKit terminated with code {} and SMC result {}.",
                io_res, smc_res
            ),
            Self::Sysctl(errno) => write!(f, "sysctl() call failed with errno {}", errno),
        }
    }
}

impl std::error::Error for SmcError {
    fn description(&self) -> &str {
        "SMC error"
    }
}

fn get_cpus_number() -> Option<usize> {
    let mut mib: [i32; 2] = [CTL_HW, HW_PACKAGES];
    let mut num: u32 = 0;
    let mut len: usize = std::mem::size_of::<u32>();

    let res = unsafe {
        sysctl(
            &mut mib[0] as *mut _,
            2,
            &mut num as *mut _ as *mut c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if res == -1 {
        None
    } else {
        Some(num as usize)
    }
}

fn get_cores_number() -> Option<usize> {
    let mut mib: [i32; 2] = [CTL_HW, HW_PHYSICALCPU];
    let mut num: u32 = 0;
    let mut len: usize = std::mem::size_of::<u32>();

    let res = unsafe {
        sysctl(
            &mut mib[0] as *mut _,
            2,
            &mut num as *mut _ as *mut c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if res == -1 {
        None
    } else {
        Some(num as usize)
    }
}

struct SmcRepr(Mutex<io_connect_t>);

impl SmcRepr {
    fn new() -> Result<Self, SmcError> {
        let conn: io_connect_t = kIOMasterPortDefault;
        let device = unsafe {
            IOServiceGetMatchingService(
                kIOMasterPortDefault,
                IOServiceMatching(b"AppleSMC\0" as *const _),
            )
        };

        if device.is_null() {
            return Err(SmcError::DriverNotFound);
        }

        let result = unsafe { IOServiceOpen(&mut *device, mach_task_self(), 0, &conn) };
        unsafe { IOObjectRelease(&mut *device) };
        if result != kIOReturnSuccess {
            return Err(SmcError::FailedToOpen);
        }

        Ok(Self(Mutex::new(conn as *mut _)))
    }

    #[allow(non_upper_case_globals)]
    fn call_driver(&self, input: &SmcParam) -> Result<SmcParam, SmcError> {
        let mut output = SmcParam::default();
        let input_size: usize = std::mem::size_of::<SmcParam>();
        let mut output_size: usize = std::mem::size_of::<SmcParam>();

        let conn = self.0.lock().unwrap();

        let result = unsafe {
            IOConnectCallStructMethod(
                *conn,
                2,
                input as *const _ as *const c_void,
                input_size,
                &mut output as *mut _ as *mut c_void,
                &mut output_size,
            )
        };

        match (result, output.result) {
            (kIOReturnSuccess, 0) => Ok(output),
            (kIOReturnSuccess, 132) => Err(SmcError::KeyNotFound(input.key)),
            (kIOReturnNotPrivileged, _) => Err(SmcError::NotPrivileged),
            _ => Err(SmcError::Unknown(result, output.result)),
        }
    }

    fn read_data<T>(&self, key: SmcKey) -> Result<T, SmcError>
    where
        T: SmcType,
    {
        let input = SmcParam {
            key: key.code,
            key_info: SmcKeyInfoData {
                data_size: key.info.size,
                ..Default::default()
            },
            selector: SmcSelector::ReadKey,
            ..Default::default()
        };
        let output = self.call_driver(&input)?;

        Ok(SmcType::from_smc(key.info, output.bytes))
    }

    fn write_data<T>(&self, key: SmcKey, data: &T) -> Result<(), SmcError>
    where
        T: SmcType,
    {
        let input = SmcParam {
            key: key.code,
            bytes: SmcType::to_smc(data, key.info),
            key_info: SmcKeyInfoData {
                data_size: key.info.size,
                ..Default::default()
            },
            selector: SmcSelector::WriteKey,
            ..Default::default()
        };

        self.call_driver(&input)?;
        Ok(())
    }

    fn key_information(&self, key: FourCharCode) -> Result<DataType, SmcError> {
        let input = SmcParam {
            key,
            selector: SmcSelector::GetKeyInfo,
            ..Default::default()
        };
        let output = self.call_driver(&input)?;

        Ok(DataType {
            id: output.key_info.data_type,
            size: output.key_info.data_size,
        })
    }

    fn read_key<T>(&self, code: FourCharCode) -> Result<T, SmcError>
    where
        T: SmcType,
    {
        let info = self.key_information(code)?;
        self.read_data(SmcKey { code, info })
    }

    fn write_key<T>(&self, code: FourCharCode, data: &T) -> Result<(), SmcError>
    where
        T: SmcType,
    {
        let info = self.key_information(code)?;
        self.write_data(SmcKey { code, info }, data)
    }

    fn key_information_at_index(&self, index: u32) -> Result<FourCharCode, SmcError> {
        let input = SmcParam {
            selector: SmcSelector::GetKeyFromIndex,
            data32: index,
            ..Default::default()
        };

        let output = self.call_driver(&input)?;
        Ok(output.key)
    }
}

impl Drop for SmcRepr {
    fn drop(&mut self) {
        let conn = self.0.lock().unwrap();
        unsafe { IOServiceClose(*conn) };
    }
}

unsafe impl Send for SmcRepr {}
unsafe impl Sync for SmcRepr {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FanMode {
    Auto,
    Forced,
}

impl FanMode {
    const fn from_bool(b: bool) -> Self {
        if b {
            Self::Forced
        } else {
            Self::Auto
        }
    }
}

impl SmcType for FanMode {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        (*self == Self::Forced).to_smc(data_type)
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        Self::from_bool(bool::from_smc(data_type, bytes))
    }
}

pub struct Fan {
    smc_repr: Arc<SmcRepr>,
    id: u32,
    name: String,
    mode: FanMode,
    min_speed: f64,
    max_speed: f64,
}

impl fmt::Debug for Fan {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Fan")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

impl Clone for Fan {
    fn clone(&self) -> Self {
        Self {
            smc_repr: self.smc_repr.clone(),
            id: self.id,
            name: self.name.clone(),
            min_speed: self.min_speed,
            max_speed: self.max_speed,
            mode: self.mode,
        }
    }
}

impl Fan {
    fn new(repr: Arc<SmcRepr>, id: u32, name: String) -> Result<Self, SmcError> {
        let mut fan = Self {
            smc_repr: repr,
            id,
            name,
            mode: FanMode::Auto,
            min_speed: 0.0,
            max_speed: 0.0,
        };

        fan.min_speed = fan.read_min_speed().unwrap_or(2500.0);
        fan.max_speed = fan.read_max_speed().unwrap_or(4000.0);
        fan.mode = fan.read_mode()?;
        Ok(fan)
    }

    #[inline]
    pub const fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn read_min_speed(&self) -> Result<f64, SmcError> {
        self.smc_repr.read_key(fcc_format!("F{}Mn", self.id))
    }

    fn read_max_speed(&self) -> Result<f64, SmcError> {
        self.smc_repr.read_key(fcc_format!("F{}Mx", self.id))
    }

    pub const fn min_speed(&self) -> f64 {
        self.min_speed
    }

    pub const fn max_speed(&self) -> f64 {
        self.max_speed
    }

    pub fn current_speed(&self) -> Result<f64, SmcError> {
        self.smc_repr.read_key(fcc_format!("F{}Ac", self.id))
    }

    pub fn rpm(&self) -> Result<f64, SmcError> {
        let rpm = self.current_speed()? - self.min_speed();

        Ok(rpm.max(0.0))
    }

    fn read_is_managed(&self) -> Result<bool, SmcError> {
        let bitmask: u16 = self.smc_repr.read_key(four_char_code!("FS! "))?;
        Ok(bitmask & (1_u16 << (self.id as u16)) == 0)
    }

    fn write_managed(&self, what: bool) -> Result<(), SmcError> {
        let bitmask: u16 = self.smc_repr.read_key(four_char_code!("FS! "))?;
        let mask = 1_u16 << (self.id as u16);
        let new: u16 = if what {
            bitmask & !mask
        } else {
            bitmask | mask
        };

        if bitmask == new {
            Ok(())
        } else {
            self.smc_repr.write_key(four_char_code!("FS! "), &new)
        }
    }

    fn read_mode(&self) -> Result<FanMode, SmcError> {
        self.smc_repr
            .read_key(fcc_format!("F{}Md", self.id))
            .or_else(|_| Ok(FanMode::from_bool(self.read_is_managed()?)))
    }

    pub fn set_mode(&mut self, mode: FanMode) -> Result<(), SmcError> {
        self.smc_repr
            .write_key(fcc_format!("F{}Md", self.id), &mode)
            .or_else(|_| self.write_managed(mode == FanMode::Auto))?;

        self.mode = mode;
        Ok(())
    }

    pub fn set_min_speed(&mut self, speed: f64) -> Result<(), SmcError> {
        if speed <= 0.0 || speed > self.max_speed {
            return Err(SmcError::UnsafeFanSpeed);
        }

        self.smc_repr
            .write_key(fcc_format!("F{}Mn", self.id), &speed)?;
        self.min_speed = speed;
        Ok(())
    }

    pub fn set_current_speed(&mut self, speed: f64) -> Result<(), SmcError> {
        if speed <= self.min_speed || speed > self.max_speed {
            return Err(SmcError::UnsafeFanSpeed);
        }

        self.set_mode(FanMode::Forced)?;
        self.smc_repr
            .write_key(fcc_format!("F{}Tg", self.id), &speed)
    }

    pub fn percent(&self) -> Result<f64, SmcError> {
        let current = self.current_speed()?;

        let rpm = current - self.min_speed;
        let rpm = if rpm < 0.0 { 0.0 } else { rpm };

        Ok(rpm / (self.max_speed - self.min_speed) * 100.0)
    }
}

unsafe impl Send for Fan {}
unsafe impl Sync for Fan {}

pub struct Smc(Arc<SmcRepr>);

impl Smc {
    pub fn new() -> Result<Self, SmcError> {
        Ok(Self(Arc::new(SmcRepr::new()?)))
    }

    fn _keys_len(&self) -> Result<u32, SmcError> {
        self.0.read_key(four_char_code!("#KEY"))
    }

    pub fn keys_len(&self) -> Result<usize, SmcError> {
        Ok(self._keys_len()? as usize)
    }

    pub fn keys(&self) -> Result<Vec<FourCharCode>, SmcError> {
        let len = self._keys_len()?;
        let mut res: Vec<FourCharCode> = Vec::with_capacity(len as usize);

        for i in 0..len {
            res.push(self.0.key_information_at_index(i)?);
        }

        Ok(res)
    }

    pub fn smc_keys(&self) -> Result<Vec<SmcKey>, SmcError> {
        let len = self._keys_len()?;
        let mut res: Vec<SmcKey> = Vec::with_capacity(len as usize);

        for i in 0..len {
            let key = self.0.key_information_at_index(i)?;
            let info = self.0.key_information(key)?;
            res.push(SmcKey { code: key, info });
        }

        Ok(res)
    }

    pub fn num_fans(&self) -> Result<usize, SmcError> {
        Ok(usize::from(self.0.read_key::<u8>(four_char_code!("FNum"))?))
    }

    fn generic_fan(&self, id: u32) -> Result<Fan, SmcError> {
        let res = self.0.read_key::<RawFan>(fcc_format!("F{}ID", id))?;

        Fan::new(self.0.clone(), id, res.name)
    }

    pub fn fan(&self, id: u32, name: Option<String>) -> Result<Fan, SmcError> {
        if let Some(name) = name {
            return Fan::new(self.0.clone(), id, name);
        }

        self.generic_fan(id)
    }

    pub fn fans(&self) -> Result<Vec<Fan>, SmcError> {
        let len = self.num_fans()?;
        let mut res: Vec<Fan> = Vec::with_capacity(len);

        for i in 0..len {
            res.push(self.generic_fan(i as u32).unwrap_or_else(|_| {
                let name = match i {
                    0 if len == 2 => "Left Fan".to_string(),
                    1 if len == 2 => "Right Fan".to_string(),
                    _ => format!("Unnamed Fan {}", i),
                };

                // SAFETY: because we pass in a Some, an Ok result is guaranteed
                unsafe { self.fan(i as u32, Some(name)).unwrap_unchecked() }
            }));
        }

        Ok(res)
    }

    pub fn temperature(&self, key: FourCharCode) -> Result<f64, SmcError> {
        if key.to_string().starts_with('T') {
            let info = self.0.key_information(key)?;

            if info.id == TYPE_SP78 || info.id == TYPE_FLT {
                self.0.read_key(key)
            } else {
                Err(SmcError::KeyNotFound(key))
            }
        } else {
            Err(SmcError::KeyNotFound(key))
        }
    }
}

impl Clone for Smc {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
