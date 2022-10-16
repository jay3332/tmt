#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation,
    clippy::module_name_repetitions,
    clippy::wildcard_imports,
    clippy::ptr_as_ptr
)]
// TODO get rid of this
#![allow(dead_code)]
#![feature(is_some_and)]

#[cfg(target_os = "macos")]
mod apple;
#[cfg(target_os = "macos")]
pub(crate) mod smc;
// #[cfg(target_os = "linux")]
mod linux;

/// The type of component.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ComponentType {
    /// A CPU component.
    Cpu,
    /// A GPU component.
    Gpu,
    /// A battery component.
    Battery,
    /// A fan component.
    Fan,
    /// A motherboard component.
    Motherboard,
    /// A generic sensor component.
    Sensor,
    /// A system component.
    System,
}

/// Common interface that represents a single temperature reading.
pub trait TemperatureReading {
    /// The label/name of what this temperature represents.
    fn label(&self) -> String;

    /// The current reading in degrees Celsius.
    fn temperature(&self) -> f64;

    /// The maximum recorded temperature in degrees Celsius.
    fn max(&self) -> f64;

    /// The temperature that will be considered "high", in degrees Celsius.
    fn high(&self) -> f64;

    /// The temperature that will be considered "critical", in degrees Celsius.
    fn critical(&self) -> f64;
}

/// Common interface that represents a temperature-measurable system component.
pub trait Component {
    type TemperatureReading: TemperatureReading;

    /// The label of the component.
    fn label(&self) -> String;

    /// The current temperature readings of the component, in celsius.
    fn temperatures(&self) -> Vec<Self::TemperatureReading>;

    /// The CPU, GPU, or battery percentage of the component, from 0.0 to 100.0.
    fn percentage(&self) -> Option<f32> {
        None
    }

    /// The type of the component.
    fn component_type(&self) -> ComponentType;

    /// Updates the component's data, if needed. By default this is a no-op.
    fn refresh(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Common interface for interacting with system components.
pub trait Interface: Default {
    /// The type of the component this interface uses.
    type Component: Component;

    /// Returns a Vec of all [`Component`]s that are eligible for thermal measurement.
    fn thermal_components(&self) -> Vec<&Self::Component>;

    /// Returns a Vec of all [`Component`]s that are eligible for thermal measurement. This one
    /// should return mutable references.
    fn thermal_components_mut(&mut self) -> Vec<&mut Self::Component>;

    /// Returns all thermal cmoponents that are of the given component type. By default this is
    /// implemented using [`Vec::into_iter`], but it can be overridden for performance boosts.
    fn thermal_components_by_type(&self, component_type: ComponentType) -> Vec<&Self::Component> {
        self.thermal_components()
            .into_iter()
            .filter(|c| c.component_type() == component_type)
            .collect()
    }

    /// Returns all thermal cmoponents that are of the given component type. By default this is
    /// implemented using [`Vec::into_iter`], but it can be overridden for performance boosts.
    /// This one should return mutable references.
    fn thermal_components_by_type_mut(
        &mut self,
        component_type: ComponentType,
    ) -> Vec<&mut Self::Component> {
        self.thermal_components_mut()
            .into_iter()
            .filter(|c| c.component_type() == component_type)
            .collect()
    }

    /// The OS name of the interface.
    fn os_name(&self) -> String;

    /// The name of the CPU or core processor.
    fn cpu_name(&self) -> String;

    /// The model of the device.
    fn device_model_name(&self) -> String;

    /// Refreshes the interface for the next iteration. By default this refreshes every component
    /// received in [`Interface::thermal_components_mut`].
    fn refresh(&mut self) -> Result<(), String> {
        for component in self.thermal_components_mut() {
            component.refresh()?;
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub use apple::AppleComponents as Provider;
#[cfg(target_os = "linux")]
pub use linux::LinuxComponents as Provider;
