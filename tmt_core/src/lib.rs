#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_possible_truncation,
    clippy::module_name_repetitions,
    clippy::wildcard_imports,
    clippy::ptr_as_ptr
)]

use std::ffi::CStr;
use std::process::Command;

#[cfg(target_os = "macos")]
mod apple;
#[cfg(target_os = "macos")]
pub(crate) mod smc;

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

/// Common interface that represents a temperature-measurable system component.
pub trait Component {
    /// The label of the component.
    fn label(&self) -> String;

    /// The current temperature of the component, in celsius.
    fn temperature(&self) -> f64;

    /// The maximum temperature of the component, in celsius.
    fn max_temperature(&self) -> Option<f64>;

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
