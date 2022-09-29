//! Uses Apple's SMC sensors to get data.

use crate::{smc, Component, ComponentType, Interface};

bitflags::bitflags! {
    /// Represents a platform compatible with a sensor.
    pub struct Platform: u8 {
        /// Compatible with Intel-based Macs.
        const INTEL = 1 << 0;
        /// Compatible with the Apple M1 SoC.
        const M1 = 1 << 1;
        /// Compatible with the Apple M1 Pro SoC.
        const M1_PRO = 1 << 2;
        /// Compatible with the Apple M1 Max SoC.
        const M1_MAX = 1 << 3;
        /// Compatible with the Apple M1 Ultra SoC.
        const M1_ULTRA = 1 << 4;
        /// Compatible with the Apple M2 SoC.
        const M2 = 1 << 5;
        /// An alias for all M1-based Macs.
        const ALL_M1 = Self::M1.bits | Self::M1_PRO.bits | Self::M1_MAX.bits | Self::M1_ULTRA.bits;
        /// An alias for all Apple Silicon-based Macs.
        const APPLE_SILICON = Self::ALL_M1.bits | Self::M2.bits;
    }
}

/// Errors raised by this module.
#[derive(Debug)]
pub enum AppleError {
    /// Error occured within Apple's SMC interface or within its bindings.
    Smc(smc::SmcError),
    /// Could not read platform information. (Sysctl error)
    Sysctl,
}

impl From<smc::SmcError> for AppleError {
    fn from(err: smc::SmcError) -> Self {
        Self::Smc(err)
    }
}

/// Represents a common group of sensors.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SensorGroup {
    /// A CPU sensor.
    Cpu,
    /// A GPU sensor.
    Gpu,
    /// A generic sensor.
    Sensor,
    /// A system sensor.
    System,
}

/// Represents a type of data that a sensor can return.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SensorKind {
    /// Measures thermal data.
    Temperature,
    /// Measures voltage.
    Voltage,
    /// Measures current.
    Current,
    /// Measures power.
    Power,
    /// Measures fan speed.
    Fan,
    /// Measures energy consumption.
    Energy,
}

/// Represents a detectable sensor.
#[derive(Copy, Clone, Debug)]
pub struct Sensor {
    /// The sensor's key.
    pub key: &'static str,
    /// The friendly name of the sensor.
    pub name: &'static str,
    /// The sensor's group of hardware.
    pub group: SensorGroup,
    /// The type of data this sensor measures or provides.
    pub kind: SensorKind,
    /// The platforms this sensor is compatible with.
    pub platforms: Platform,
    /// Whether this sensor is calculated by an average of other sensors.
    pub average: bool,
    /// The component type of this sensor.
    pub component_type: ComponentType,
}

macro_rules! impl_sensor_group {
    ($name:ident $variant:ident) => {
        const fn $name(
            key: &'static str,
            name: &'static str,
            kind: SensorKind,
            platforms: Platform,
        ) -> Self {
            Self {
                key,
                name,
                group: SensorGroup::$variant,
                kind,
                platforms,
                average: false,
                component_type: ComponentType::$variant,
            }
        }
    };
}

#[allow(clippy::self_named_constructors)]
impl Sensor {
    impl_sensor_group!(cpu Cpu);
    impl_sensor_group!(gpu Gpu);
    impl_sensor_group!(sensor Sensor);
    impl_sensor_group!(system System);

    const fn average(mut self) -> Self {
        self.average = true;
        self
    }

    const fn component_type(mut self, kind: ComponentType) -> Self {
        self.component_type = kind;
        self
    }
}

macro_rules! apple_component {
    ($($variant:ident $t:ty),+) => {
        pub enum AppleComponent {
            $(
                $variant($t),
            )+
        }

        impl Component for AppleComponent {
            fn label(&self) -> String {
               match self {
                   $(
                       Self::$variant(component) => component.label(),
                   )+
               }
            }

            fn temperature(&self) -> f64 {
                match self {
                    $(
                        Self::$variant(component) => component.temperature(),
                    )+
                }
            }

            fn max_temperature(&self) -> Option<f64> {
                match self {
                    $(
                        Self::$variant(component) => component.max_temperature(),
                    )+
                }
            }

            fn component_type(&self) -> ComponentType {
                match self {
                    $(
                        Self::$variant(component) => component.component_type(),
                    )+
                }
            }

            fn refresh(&mut self) -> Result<(), String> {
                match self {
                    $(
                        Self::$variant(component) => component.refresh(),
                    )+
                }
            }
        }
    };
}

apple_component! {
    Cpu AppleCpuComponent,
    Gpu AppleGpuComponent
}

macro_rules! xpu_component_impl {
    ($($t:ident)+) => {
        $(
            pub struct $t {
                smc: smc::Smc,
                inner: Sensor,
                previous: f64,
                max: f64,
            }

            impl Component for $t {
                fn label(&self) -> String {
                    self.inner.name.to_string()
                }

                fn temperature(&self) -> f64 {
                    self.previous
                }

                fn max_temperature(&self) -> Option<f64> {
                    Some(self.max)
                }

                fn component_type(&self) -> ComponentType {
                    ComponentType::Cpu
                }

                fn refresh(&mut self) -> Result<(), String> {
                    self.previous = self
                        .smc
                        .temperature(self.inner.key.into())
                        .map_err(|e| e.to_string())?;
                    self.max = self.max.max(self.previous);

                    Ok(())
                }
            }
        )+
    }
}

xpu_component_impl!(AppleCpuComponent AppleGpuComponent);

pub struct AppleComponents {
    smc: smc::Smc,
    sensors: Vec<(Sensor, AppleComponent)>,
}

impl AppleComponents {
    fn new() -> Result<Self, AppleError> {
        let smc = smc::Smc::new()?;
        let keys = smc.keys()?;
        let processor = unsafe { read_processor().ok_or(AppleError::Sysctl)? };
        let sensors = SENSORS
            .into_iter()
            .filter_map(|sensor| {
                if keys.contains(&sensor.key.into()) && sensor.platforms.contains(processor) {
                    let mut component = match sensor.component_type {
                        ComponentType::Cpu => AppleComponent::Cpu(AppleCpuComponent {
                            smc: smc.clone(),
                            inner: sensor,
                            previous: 0.0,
                            max: 0.0,
                        }),
                        ComponentType::Gpu => AppleComponent::Gpu(AppleGpuComponent {
                            smc: smc.clone(),
                            inner: sensor,
                            previous: 0.0,
                            max: 0.0,
                        }),
                        _ => return None,
                    };

                    component.refresh().ok().map(|_| (sensor, component))
                } else {
                    None
                }
            })
            .collect();

        Ok(Self { smc, sensors })
    }
}

impl Interface for AppleComponents {
    type Component = AppleComponent;

    fn thermal_components(&self) -> Vec<&Self::Component> {
        self.sensors
            .iter()
            .filter_map(|(s, c)| (s.kind == SensorKind::Temperature).then_some(c))
            .collect()
    }

    fn thermal_components_mut(&mut self) -> Vec<&mut Self::Component> {
        self.sensors
            .iter_mut()
            .filter_map(|(s, c)| (s.kind == SensorKind::Temperature).then_some(c))
            .collect()
    }

    fn os_name(&self) -> String {
        todo!()
    }
}

impl Default for AppleComponents {
    fn default() -> Self {
        Self::new().expect("could not init SMC: are you running as root?")
    }
}

unsafe fn read_processor() -> Option<Platform> {
    let mut size = 0usize;
    let key = std::ffi::CString::new("machdep.cpu.brand_string").unwrap();

    let res = libc::sysctlbyname(
        key.as_ptr(),
        std::ptr::null_mut(),
        &mut size,
        std::ptr::null_mut(),
        0,
    );
    if res != 0 {
        return None;
    }

    // Allow up to 24 characters for the processor name. Since we're only checking for the Apple
    // Silicon processors, this should be enough: Apple MXX XXXXXXXXXXXXXXX
    let mut chars = [0_i8; 24];

    libc::sysctlbyname(
        key.as_ptr(),
        chars.as_mut_ptr() as *mut _,
        &mut size,
        std::ptr::null_mut(),
        0,
    );
    if res != 0 {
        return None;
    }

    let chars = &std::mem::transmute::<_, [u8; 24]>(chars);
    let name = String::from_utf8_lossy(chars);
    let name = name.trim_end_matches('\0');

    if !name.starts_with("Apple M") {
        return Some(Platform::INTEL);
    }

    // SAFETY: already checked that the name starts with "Apple M"
    Some(match name.strip_prefix("Apple M").unwrap_unchecked() {
        "1" => Platform::M1,
        "1 Pro" => Platform::M1_PRO,
        "1 Max" => Platform::M1_MAX,
        "1 Ultra" => Platform::M1_ULTRA,
        "2" => Platform::M2,
        _ => Platform::INTEL,
    })
}

/// A collection of known sensors.
pub const SENSORS: [Sensor; 94] = [
    // Generic temperature sensors
    Sensor::sensor(
        "TA%P",
        "Ambient %",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::sensor(
        "Th%H",
        "Heatpipe %",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::sensor(
        "TZ%C",
        "Thermal Zone %",
        SensorKind::Temperature,
        Platform::all(),
    ),
    // CPU temperature sensors
    Sensor::cpu(
        "TC0D",
        "CPU diode",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::cpu(
        "TC0E",
        "CPU diode virtual",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::cpu(
        "TC0F",
        "CPU diode filtered",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::cpu(
        "TC0H",
        "CPU heatsink",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::cpu(
        "TC0P",
        "CPU proximity",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::cpu(
        "TCAD",
        "CPU package",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::cpu(
        "TC%c",
        "CPU Core %",
        SensorKind::Temperature,
        Platform::all(),
    )
    .average(),
    Sensor::cpu(
        "TC%C",
        "CPU Core %",
        SensorKind::Temperature,
        Platform::all(),
    )
    .average(),
    // GPU temperature sensors
    Sensor::gpu(
        "TCGC",
        "GPU Intel Graphics",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::gpu(
        "TG0D",
        "GPU diode",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::gpu(
        "TGDD",
        "GPU AMD Radeon",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::gpu(
        "TG0H",
        "GPU heatsink",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::gpu(
        "TG0P",
        "GPU proximity",
        SensorKind::Temperature,
        Platform::all(),
    ),
    // System temperature sensors
    Sensor::system(
        "Tm0P",
        "Mainboard",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::system(
        "Tp0P",
        "Powerboard",
        SensorKind::Temperature,
        Platform::INTEL,
    ),
    Sensor::system("TB1T", "Battery", SensorKind::Temperature, Platform::INTEL),
    Sensor::system("TW0P", "Airport", SensorKind::Temperature, Platform::all()),
    Sensor::system("TL0P", "Display", SensorKind::Temperature, Platform::all()),
    Sensor::system(
        "TI%P",
        "Thunderbolt %",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::system(
        "TH%A",
        "Disk % (A)",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::system(
        "TH%B",
        "Disk % (B)",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::system(
        "TH%C",
        "Disk % (C)",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::system(
        "TN0D",
        "Northbridge diode",
        SensorKind::Temperature,
        Platform::all(),
    )
    .component_type(ComponentType::Motherboard),
    Sensor::system(
        "TN0H",
        "Northbridge heatsink",
        SensorKind::Temperature,
        Platform::all(),
    )
    .component_type(ComponentType::Motherboard),
    Sensor::system(
        "TN0P",
        "Northbridge proximity",
        SensorKind::Temperature,
        Platform::all(),
    )
    .component_type(ComponentType::Motherboard),
    // M1 series CPU temperature sensors
    Sensor::cpu(
        "Tp09",
        "CPU efficiency core 1",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp0T",
        "CPU efficiency core 2",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp01",
        "CPU performance core 1",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp05",
        "CPU performance core 2",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp0D",
        "CPU performance core 3",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp0H",
        "CPU performance core 4",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp0L",
        "CPU performance core 5",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp0P",
        "CPU performance core 6",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp0X",
        "CPU performance core 7",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::cpu(
        "Tp0b",
        "CPU performance core 8",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    // M1 series GPU temperature sensors
    Sensor::gpu(
        "Tg05",
        "GPU Cluster 1",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::gpu(
        "Tg0D",
        "GPU Cluster 2",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::gpu(
        "Tg0L",
        "GPU Cluster 3",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    Sensor::gpu(
        "Tg0T",
        "GPU Cluster 4",
        SensorKind::Temperature,
        Platform::ALL_M1,
    )
    .average(),
    // M2 series CPU temperature sensors
    Sensor::cpu(
        "Tp05",
        "CPU efficiency core 1",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::cpu(
        "Tp0D",
        "CPU efficiency core 2",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::cpu(
        "Tp0j",
        "CPU efficiency core 3",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::cpu(
        "Tp0r",
        "CPU efficiency core 4",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::cpu(
        "Tp01",
        "CPU performance core 1",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::cpu(
        "Tp09",
        "CPU performance core 2",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::cpu(
        "Tp0f",
        "CPU performance core 3",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::cpu(
        "Tp0n",
        "CPU performance core 4",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    // M2 series GPU temperature sensors
    Sensor::gpu(
        "Tg0f",
        "GPU Cluster 1",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    Sensor::gpu(
        "Tg0n",
        "GPU Cluster 2",
        SensorKind::Temperature,
        Platform::M2,
    )
    .average(),
    // Other hardware temperature sensors
    Sensor::system(
        "TaLP",
        "Airflow left",
        SensorKind::Temperature,
        Platform::APPLE_SILICON,
    ),
    Sensor::system(
        "TaRF",
        "Airflow right",
        SensorKind::Temperature,
        Platform::APPLE_SILICON,
    ),
    Sensor::system(
        "TH0x",
        "NAND",
        SensorKind::Temperature,
        Platform::APPLE_SILICON,
    ),
    Sensor::system(
        "TB1T",
        "Battery 1",
        SensorKind::Temperature,
        Platform::APPLE_SILICON,
    )
    .component_type(ComponentType::Battery),
    Sensor::system(
        "TB2T",
        "Battery 2",
        SensorKind::Temperature,
        Platform::APPLE_SILICON,
    )
    .component_type(ComponentType::Battery),
    Sensor::system(
        "TW0P",
        "Airport",
        SensorKind::Temperature,
        Platform::APPLE_SILICON,
    ),
    // Voltage sensors
    Sensor::system("VCAC", "CPU IA", SensorKind::Voltage, Platform::all()),
    Sensor::system(
        "VCSC",
        "CPU System Agent",
        SensorKind::Voltage,
        Platform::all(),
    ),
    Sensor::system("VC%C", "CPU Core %", SensorKind::Voltage, Platform::all()),
    Sensor::system(
        "VCTC",
        "GPU Intel Graphics",
        SensorKind::Voltage,
        Platform::all(),
    ),
    Sensor::system("VG0C", "GPU", SensorKind::Voltage, Platform::all()),
    Sensor::system("VM0R", "Memory", SensorKind::Voltage, Platform::all()),
    Sensor::system("Vb0R", "CMOS", SensorKind::Voltage, Platform::all()),
    Sensor::system("VD0R", "DC In", SensorKind::Voltage, Platform::all()),
    Sensor::system("VP0R", "12V rail", SensorKind::Voltage, Platform::all()),
    Sensor::system("Vp0C", "12V vcc", SensorKind::Voltage, Platform::all()),
    Sensor::system("VV2S", "3V", SensorKind::Voltage, Platform::all()),
    Sensor::system("VR3R", "3.3V", SensorKind::Voltage, Platform::all()),
    Sensor::system("VV1S", "5V", SensorKind::Voltage, Platform::all()),
    Sensor::system("VV9S", "12V", SensorKind::Voltage, Platform::all()),
    Sensor::system("VeES", "PCI 12V", SensorKind::Voltage, Platform::all()),
    // Current
    Sensor::sensor(
        "IC0R",
        "CPU High side",
        SensorKind::Current,
        Platform::all(),
    ),
    Sensor::sensor(
        "IG0R",
        "GPU High side",
        SensorKind::Current,
        Platform::all(),
    ),
    Sensor::sensor("ID0R", "DC In", SensorKind::Current, Platform::all()),
    Sensor::sensor("IBAC", "Battery", SensorKind::Current, Platform::all()),
    // Power
    Sensor::sensor("PC0C", "CPU Core", SensorKind::Power, Platform::all()),
    Sensor::sensor(
        "PCAM",
        "CPU Core (IMON)",
        SensorKind::Power,
        Platform::all(),
    ),
    Sensor::sensor("PCPC", "CPU Package", SensorKind::Power, Platform::all()),
    Sensor::sensor("PCTR", "CPU Total", SensorKind::Power, Platform::all()),
    Sensor::sensor(
        "PCPT",
        "CPU Package total",
        SensorKind::Power,
        Platform::all(),
    ),
    Sensor::sensor(
        "PCPR",
        "CPU Package total (SMC)",
        SensorKind::Power,
        Platform::all(),
    ),
    Sensor::sensor(
        "PC0R",
        "CPU Computing high side",
        SensorKind::Power,
        Platform::all(),
    ),
    Sensor::sensor("PC0G", "CPU GFX", SensorKind::Power, Platform::all()),
    Sensor::sensor("PCEC", "CPU VccEDRAM", SensorKind::Power, Platform::all()),
    Sensor::sensor(
        "PCPG",
        "GPU Intel Graphics",
        SensorKind::Power,
        Platform::all(),
    ),
    Sensor::sensor("PG0R", "GPU", SensorKind::Power, Platform::all()),
    Sensor::sensor("PCGC", "Intel GPU", SensorKind::Power, Platform::all()),
    Sensor::sensor(
        "PCGM",
        "Intel GPU (IMON)",
        SensorKind::Power,
        Platform::all(),
    ),
    Sensor::sensor("PC3C", "RAM", SensorKind::Power, Platform::all()),
    Sensor::sensor("PPBR", "Battery", SensorKind::Power, Platform::all())
        .component_type(ComponentType::Battery),
    Sensor::sensor("PDTR", "DC In", SensorKind::Power, Platform::all()),
    Sensor::sensor("PSTR", "System Total", SensorKind::Power, Platform::all()),
];
