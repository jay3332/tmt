//! Uses Apple's SMC sensors to get data.

use four_char_code::FourCharCode;

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
            }
        }
    };
}

impl Sensor {
    impl_sensor_group!(cpu Cpu);
    impl_sensor_group!(gpu Gpu);
    impl_sensor_group!(sensor Sensor);
    impl_sensor_group!(system System);

    const fn average(mut self) -> Self {
        self.average = true;
        self
    }
}

pub fn get_all_sensors() -> Result<Vec<Sensor>, smc::SMCError> {
    let keys = smc::SMC::new()?.keys()?;

    Ok(SENSORS
        .into_iter()
        .filter(|sensor| keys.contains(&FourCharCode::from(sensor.key)))
        .collect())
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
    ),
    Sensor::system(
        "TN0H",
        "Northbridge heatsink",
        SensorKind::Temperature,
        Platform::all(),
    ),
    Sensor::system(
        "TN0P",
        "Northbridge proximity",
        SensorKind::Temperature,
        Platform::all(),
    ),
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
    ),
    Sensor::system(
        "TB2T",
        "Battery 2",
        SensorKind::Temperature,
        Platform::APPLE_SILICON,
    ),
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
    Sensor::sensor("PPBR", "Battery", SensorKind::Power, Platform::all()),
    Sensor::sensor("PDTR", "DC In", SensorKind::Power, Platform::all()),
    Sensor::sensor("PSTR", "System Total", SensorKind::Power, Platform::all()),
];
