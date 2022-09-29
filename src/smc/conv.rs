use super::{DataType, SmcBytes};

#[derive(Debug)]
pub struct RawFan {
    pub name: String,
}

use four_char_code::{four_char_code, FourCharCode};
use libc::{c_void, memcpy};
use std::{slice, str};

const TYPE_FLAG: FourCharCode = four_char_code!("flag");
const TYPE_I8: FourCharCode = four_char_code!("si8 ");
const TYPE_U8: FourCharCode = four_char_code!("ui8 ");
const TYPE_I16: FourCharCode = four_char_code!("si16");
const TYPE_U16: FourCharCode = four_char_code!("ui16");
const TYPE_I32: FourCharCode = four_char_code!("si32");
const TYPE_U32: FourCharCode = four_char_code!("ui32");
const TYPE_FPE2: FourCharCode = four_char_code!("fpe2");
const TYPE_SP78: FourCharCode = four_char_code!("sp78");
const TYPE_FAN: FourCharCode = four_char_code!("{fds");
const TYPE_FLT: FourCharCode = four_char_code!("flt ");

fn read_string(buffer: *const u8, max: usize) -> String {
    let len = unsafe { slice::from_raw_parts(buffer, max) }
        .iter()
        .position(|v| *v == 0)
        .map_or(max, |pos| pos);

    unsafe { str::from_utf8_unchecked(slice::from_raw_parts(buffer, len)) }
        .trim()
        .to_string()
}

pub trait SmcType {
    fn to_smc(&self, data_type: DataType) -> SmcBytes;
    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self;
}

impl SmcType for bool {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        if data_type.id == TYPE_FLAG || data_type.id == TYPE_U8 {
            let mut res = SmcBytes::default();
            res.0[0] = u8::from(*self);
            res
        } else {
            panic!("Cannot convert bool to {:?}", data_type);
        }
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_FLAG || data_type.id == TYPE_U8 {
            bytes.0[0] != 0
        } else {
            panic!("Cannot convert {:?} to bool", data_type);
        }
    }
}

impl SmcType for i8 {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        if data_type.id == TYPE_I8 {
            let mut res = SmcBytes::default();
            unsafe {
                memcpy(
                    &mut res as *mut _ as *mut c_void,
                    self as *const _ as *const c_void,
                    std::mem::size_of::<Self>(),
                );
            }
            res
        } else {
            panic!("Cannot convert i8 to {:?}", data_type);
        }
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_I8 {
            unsafe { *(&(bytes.0[0]) as *const _ as *const Self) }
        } else {
            panic!("Cannot convert {:?} to i8", data_type);
        }
    }
}

impl SmcType for u8 {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        if data_type.id == TYPE_U8 {
            let mut res = SmcBytes::default();
            res.0[0] = *self;
            res
        } else {
            panic!("Cannot convert u8 to {:?}", data_type);
        }
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_U8 {
            bytes.0[0]
        } else {
            panic!("Cannot convert {:?} to u8", data_type);
        }
    }
}

impl SmcType for i16 {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        if data_type.id == TYPE_I16 {
            let mut res = SmcBytes::default();
            unsafe {
                memcpy(
                    &mut res as *mut _ as *mut c_void,
                    &self.to_be() as *const _ as *const c_void,
                    std::mem::size_of::<Self>(),
                );
            }
            res
        } else {
            panic!("Cannot convert i16 to {:?}", data_type);
        }
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_I16 {
            Self::from_be(unsafe { *(&(bytes.0[0]) as *const _ as *const Self) })
        } else {
            panic!("Cannot convert {:?} to i16", data_type);
        }
    }
}

impl SmcType for u16 {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        if data_type.id == TYPE_U16 {
            let mut res = SmcBytes::default();
            unsafe {
                memcpy(
                    &mut res as *mut _ as *mut c_void,
                    &self.to_be() as *const _ as *const c_void,
                    std::mem::size_of::<Self>(),
                );
            }
            res
        } else {
            panic!("Cannot convert u16 to {:?}", data_type);
        }
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_U8 {
            Self::from(<u8 as SmcType>::from_smc(data_type, bytes))
        } else if data_type.id == TYPE_U16 {
            Self::from_be(unsafe { *(&(bytes.0[0]) as *const _ as *const Self) })
        } else {
            panic!("Cannot convert {:?} to u16", data_type);
        }
    }
}

impl SmcType for i32 {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        if data_type.id == TYPE_I32 {
            let mut res = SmcBytes::default();
            unsafe {
                memcpy(
                    &mut res as *mut _ as *mut c_void,
                    &self.to_be() as *const _ as *const c_void,
                    std::mem::size_of::<Self>(),
                );
            }
            res
        } else {
            panic!("Cannot convert i32 to {:?}", data_type);
        }
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_I32 {
            Self::from_be(unsafe { *(&(bytes.0[0]) as *const _ as *const Self) })
        } else {
            panic!("Cannot convert {:?} to i32", data_type);
        }
    }
}

impl SmcType for u32 {
    fn to_smc(&self, data_type: DataType) -> SmcBytes {
        if data_type.id == TYPE_U32 {
            let mut res = SmcBytes::default();
            unsafe {
                memcpy(
                    &mut res as *mut _ as *mut c_void,
                    &self.to_be() as *const _ as *const c_void,
                    std::mem::size_of::<Self>(),
                );
            }
            res
        } else {
            panic!("Cannot convert u32 to {:?}", data_type);
        }
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_U8 {
            Self::from(<u8 as SmcType>::from_smc(data_type, bytes))
        } else if data_type.id == TYPE_U16 {
            Self::from(<u16 as SmcType>::from_smc(data_type, bytes))
        } else if data_type.id == TYPE_U32 {
            Self::from_be(unsafe { *(&(bytes.0[0]) as *const _ as *const Self) })
        } else {
            panic!("Cannot convert {:?} to u32", data_type);
        }
    }
}

impl SmcType for RawFan {
    fn to_smc(&self, _data_type: DataType) -> SmcBytes {
        panic!("You can't write a RawFan type");
    }

    fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
        if data_type.id == TYPE_FAN {
            let name = read_string(
                unsafe { (&bytes.0[0] as *const u8).add(4) },
                (data_type.size - 4) as usize,
            );
            Self { name }
        } else {
            panic!("Cannot convert {:?} to RawFan", data_type);
        }
    }
}

macro_rules! def_float {
    ($t:ty) => {
        impl SmcType for $t {
            fn to_smc(&self, data_type: DataType) -> SmcBytes {
                if data_type.id == TYPE_FPE2 {
                    if self.is_sign_negative() {
                        panic!(concat!(
                            "Cannot convert negative ",
                            stringify!($t),
                            " to fpe2"
                        ));
                    }

                    let value = ((self * 4.0) as u16).to_be();

                    let mut res = SmcBytes::default();
                    unsafe {
                        memcpy(
                            &mut res as *mut _ as *mut c_void,
                            &value as *const _ as *const c_void,
                            std::mem::size_of::<u16>(),
                        );
                    }
                    res
                } else if data_type.id == TYPE_SP78 {
                    let value = ((self * 256.0) as i16).to_be();

                    let mut res = SmcBytes::default();
                    unsafe {
                        memcpy(
                            &mut res as *mut _ as *mut c_void,
                            &value as *const _ as *const c_void,
                            std::mem::size_of::<u16>(),
                        );
                    }
                    res
                } else if data_type.id == TYPE_FLT {
                    let mut buf: [u8; 32] = Default::default();
                    let bytes = (*self as f32).to_ne_bytes();
                    buf[0] = bytes[0];
                    buf[1] = bytes[1];
                    buf[2] = bytes[2];
                    buf[3] = bytes[3];

                    SmcBytes(buf)
                } else {
                    panic!(
                        concat!("Cannot convert ", stringify!($t), " to {:?}"),
                        data_type
                    );
                }
            }

            fn from_smc(data_type: DataType, bytes: SmcBytes) -> Self {
                if data_type.id == TYPE_FPE2 {
                    (u16::from_be(unsafe { *(&bytes.0[0] as *const _ as *const u16) }) as Self)
                        / 4.0
                } else if data_type.id == TYPE_SP78 {
                    (i16::from_be(unsafe { *(&bytes.0[0] as *const _ as *const i16) }) as Self)
                        / 256.0
                } else if data_type.id == TYPE_FLT {
                    let mut buf: [u8; 4] = Default::default();
                    let shortened = &bytes.0[..4];
                    buf.copy_from_slice(shortened);
                    f32::from_ne_bytes(buf) as Self
                } else {
                    panic!(
                        concat!("Cannot convert {:?} to ", stringify!($t)),
                        data_type
                    );
                }
            }
        }
    };
}

def_float!(f32);
def_float!(f64);
