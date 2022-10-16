use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use self::LinuxError::InvalidData;
use super::{Component, ComponentType, Interface, TemperatureReading as TemperatureReadingTrait};

/// An error that occured in this module.
#[derive(Debug)]
pub enum LinuxError {
    /// IO error occured when reading from hwmon.
    IoError(std::io::Error),
    /// Invalid data received from hwmon.
    InvalidData(String),
}

impl std::error::Error for LinuxError {}

impl std::fmt::Display for LinuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&match self {
            Self::IoError(err) => format!("io error: {}", err),
            InvalidData(msg) => format!("invalid data: {}", msg),
        })
    }
}

impl From<std::io::Error> for LinuxError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

/// The sensor type read from /sys/class/hwmon/hwmon*/temp*_type
#[derive(Copy, Clone, Debug)]
pub enum HwmonSensorType {
    /// CPU embedded diode
    CpuDiode,
    /// 3904 transistor
    Transistor,
    /// Thermal diode
    ThermalDiode,
    /// Thermistor
    Thermistor,
    /// AMD AMDSI
    Amdsi,
    /// Intel PECI
    Peci,
}

impl HwmonSensorType {
    /// Returns the sensor type from the given string slice
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "1" => Some(HwmonSensorType::CpuDiode),
            "2" => Some(HwmonSensorType::Transistor),
            "3" => Some(HwmonSensorType::ThermalDiode),
            "4" => Some(HwmonSensorType::Thermistor),
            "5" => Some(HwmonSensorType::Amdsi),
            "6" => Some(HwmonSensorType::Peci),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TemperatureReading {
    pub name: String,
    pub temperature: u32,
    pub max: u32,
    pub high: u32,
    pub crit: u32,
}

impl TemperatureReadingTrait for TemperatureReading {
    fn label(&self) -> String {
        self.name.clone()
    }

    fn temperature(&self) -> f64 {
        self.temperature as f64 / 1000.0
    }

    fn max(&self) -> f64 {
        self.max as f64 / 1000.0
    }

    fn high(&self) -> f64 {
        self.high as f64 / 1000.0
    }

    fn critical(&self) -> f64 {
        self.crit as f64 / 1000.0
    }
}

pub struct HwmonSensor {
    path: PathBuf,
    device_path: PathBuf,
    name: Option<String>,
    update_interval: Duration,
    last_update: Instant,
    readings: HashMap<String, TemperatureReading>,
    sensor_type: HwmonSensorType,
    wait: bool,
}

impl HwmonSensor {
    #[must_use]
    fn new(
        path: PathBuf,
        device_path: PathBuf,
        name: Option<String>,
        update_interval: Duration,
        sensor_type: HwmonSensorType,
    ) -> Self {
        Self {
            path,
            device_path,
            name,
            update_interval,
            sensor_type,
            last_update: Instant::now(),
            wait: false,
            readings: HashMap::new(),
        }
    }

    fn should_read(&self) -> bool {
        if self.wait && self.last_update.elapsed() < self.update_interval {
            return false;
        }

        let power_state = self.device_path.join("power_state");

        if power_state.exists() {
            let state = std::fs::read_to_string(power_state).unwrap_or_default();
            let state = state.trim();

            state == "D0" || state == "unknown"
        } else {
            true
        }
    }

    fn read_temperatures(&mut self) -> Result<(), LinuxError> {
        if !self.should_read() {
            return Ok(());
        }

        for entry in self.path.read_dir()? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or(InvalidData(
                "hwmon sensor had a non-ascii device filename".to_string(),
            ))?;

            if name.starts_with("temp") && name.ends_with("_input") {
                let temperature = std::fs::read_to_string(entry.path())?;
                let temperature = temperature.trim().parse::<u32>().map_err(|_| {
                    InvalidData(format!("read invalid temperature {}", temperature))
                })?;

                macro_rules! read {
                    ($field:literal) => {{
                        let name = name.replace("_input", concat!("_", $field));
                        let name = self.path.join(name);

                        std::fs::read_to_string(name).ok()
                    }};
                }

                let name = match (&self.name, read!("label")) {
                    (Some(name), Some(label)) => format!("{}: {}", name, label),
                    (Some(name), None) => name.clone(),
                    (None, Some(label)) => label,
                    (None, None) => "Unknown".to_string(),
                };

                let max = read!("highest")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(0);

                let high = read!("max")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(85_000);

                let crit = read!("crit")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(100_000);

                self.readings.insert(
                    name.clone(),
                    TemperatureReading {
                        name,
                        temperature,
                        max,
                        high,
                        crit,
                    },
                );
            }
        }

        self.last_update = Instant::now();
        self.wait = true;

        Ok(())
    }
}

pub struct ThermalZoneSensor {
    path: PathBuf,
    name: String,
    last_reading: Option<u32>,
    max: u32,
    high: u32,
    crit: u32,
}

impl ThermalZoneSensor {
    fn read_temperature(&mut self) -> Result<(), LinuxError> {
        let temperature = std::fs::read_to_string(self.path.join("temp"))?;
        self.last_reading = Some(
            temperature
                .trim()
                .parse::<u32>()
                .map_err(|_| InvalidData(format!("read invalid temperature {}", temperature)))?,
        );

        Ok(())
    }
}

fn get_sensors_from_hwmon() -> Result<Vec<HwmonSensor>, LinuxError> {
    let mut sensors = Vec::new();
    let path = Path::new("/sys/class/hwmon");

    for entry in path.read_dir()? {
        let entry = entry?;
        let mut file_path = entry.path();

        // Check that at least one temperature sensor exists
        if !file_path.join("temp1_input").exists() {
            if file_path.join("device/temp1_input").exists() {
                file_path = file_path.join("device");
            } else {
                continue;
            }
        }

        let name = std::fs::read_to_string(file_path.join("name")).ok();

        let update_interval = std::fs::read_to_string(file_path.join("update_interval"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or_default();

        let sensor_type = std::fs::read_to_string(file_path.join("temp1_type"))
            .ok()
            .and_then(|s| HwmonSensorType::from_str(&s))
            .unwrap_or(HwmonSensorType::CpuDiode);

        sensors.push(HwmonSensor::new(
            file_path,
            entry.path().join("device"),
            name,
            update_interval,
            sensor_type,
        ));
    }

    Ok(sensors)
}

fn get_sensors_from_thermal_zone() -> Result<Vec<ThermalZoneSensor>, LinuxError> {
    let mut sensors = Vec::new();
    let path = Path::new("/sys/class/thermal");

    for entry in path.read_dir()? {
        let entry = entry?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|e| e.starts_with("thermal_zone"))
        {
            continue;
        }

        let name = std::fs::read_to_string(entry.path().join("type"))?;
        let name = name.trim().to_string();

        let (mut high, mut critical) = (85_000, 100_000);

        for entry in entry.path().read_dir()? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or(InvalidData(
                "thermal zone sensor had a non-ascii device filename".to_string(),
            ))?;

            if !name.starts_with("trip_point_") || !name.ends_with("_temp") {
                continue;
            }

            let temperature = std::fs::read_to_string(entry.path())?;
            let temperature = temperature
                .trim()
                .parse::<u32>()
                .map_err(|_| InvalidData(format!("read invalid temperature {}", temperature)))?;

            let name = name.replace("_temp", "_type");
            let name = entry.path().parent().unwrap().join(name);

            match std::fs::read_to_string(name)
                .ok()
                .unwrap_or_default()
                .trim()
            {
                "critical" => critical = temperature,
                "hot" => high = temperature,
                _ => continue,
            }
        }

        sensors.push(ThermalZoneSensor {
            path: entry.path(),
            name,
            last_reading: None,
            max: 0,
            high,
            crit: critical,
        });
    }

    Ok(sensors)
}

pub enum LinuxHardwareComponent {
    Hwmon(HwmonSensor),
    ThermalZone(ThermalZoneSensor),
}

impl Component for LinuxHardwareComponent {
    type TemperatureReading = TemperatureReading;

    fn label(&self) -> String {
        match self {
            Self::Hwmon(sensor) => sensor.name.as_ref().unwrap().clone(),
            Self::ThermalZone(sensor) => sensor.name.clone(),
        }
    }

    fn temperatures(&self) -> Vec<Self::TemperatureReading> {
        match self {
            Self::Hwmon(sensor) => sensor.readings.values().cloned().collect(),
            Self::ThermalZone(sensor) => sensor
                .last_reading
                .map(|temperature| {
                    vec![TemperatureReading {
                        name: sensor.name.clone(),
                        temperature,
                        max: sensor.max,
                        high: sensor.high,
                        crit: sensor.crit,
                    }]
                })
                .unwrap_or_default(),
        }
    }

    fn component_type(&self) -> ComponentType {
        // TODO (this is a placeholder)
        ComponentType::Cpu
    }

    fn refresh(&mut self) -> Result<(), String> {
        match self {
            Self::Hwmon(sensor) => sensor.read_temperatures().map_err(|e| e.to_string()),
            Self::ThermalZone(sensor) => sensor.read_temperature().map_err(|e| e.to_string()),
        }
    }
}

fn get_temperature_sensors() -> Result<Vec<LinuxHardwareComponent>, LinuxError> {
    // TODO There might be cases where it's useful to *combine* hwmon and thermal zone sensors
    // TODO instead of making thermal zone sensors a fallback.
    let sensors = get_sensors_from_hwmon()?;

    if sensors.is_empty() {
        let sensors = get_sensors_from_thermal_zone()?;
        Ok(sensors
            .into_iter()
            .map(LinuxHardwareComponent::ThermalZone)
            .collect())
    } else {
        Ok(sensors
            .into_iter()
            .map(LinuxHardwareComponent::Hwmon)
            .collect())
    }
}

pub struct LinuxComponents {
    sensors: Vec<LinuxHardwareComponent>,
}

impl LinuxComponents {
    pub fn new() -> Result<Self, LinuxError> {
        let sensors = get_temperature_sensors()?;

        Ok(LinuxComponents { sensors })
    }
}

impl Interface for LinuxComponents {
    type Component = LinuxHardwareComponent;

    fn thermal_components(&self) -> Vec<&Self::Component> {
        self.sensors.iter().collect()
    }

    fn thermal_components_mut(&mut self) -> Vec<&mut Self::Component> {
        self.sensors.iter_mut().collect()
    }

    fn os_name(&self) -> String {
        OS_NAME.clone()
    }

    fn cpu_name(&self) -> String {
        CPU_NAME.clone()
    }

    fn device_model_name(&self) -> String {
        DEVICE_NAME.clone()
    }
}

impl Default for LinuxComponents {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

fn get_os_release_key(key: &'static str) -> String {
    let path = Path::new("/etc/os-release");
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = line.unwrap();

        if line.starts_with(key) {
            let value = line.split('=').nth(1).unwrap();
            return value.trim_matches('"').to_string();
        }
    }

    panic!(
        "os-release: could not find key {:?} in {}",
        key,
        path.display()
    );
}

fn get_processor_key(id: u8, key: &'static str) -> String {
    let path = Path::new("/proc/cpuinfo");
    let file = std::fs::File::open(path).unwrap();
    let reader = std::io::BufReader::new(file);

    let mut cpu_id = 0;
    for line in reader.lines() {
        let line = line.unwrap();

        if line.starts_with("processor") {
            cpu_id = line.split(':').nth(1).unwrap().trim().parse().unwrap();
        }

        if cpu_id == id && line.starts_with(key) {
            let value = line.split(':').nth(1).unwrap();
            return value.trim().to_string();
        }
    }

    panic!("cpuinfo: could not find key {:?} in processor {}", key, id);
}

lazy_static::lazy_static! {
    static ref OS_NAME: String = get_os_release_key("PRETTY_NAME");
    static ref DEVICE_NAME: String = std::fs::read_to_string("/sys/devices/virtual/dmi/id/product_name")
        .unwrap_or_else(|_| "Unknown".to_string());
    static ref CPU_NAME: String = get_processor_key(0, "model name");
}
