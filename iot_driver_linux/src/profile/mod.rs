// Device profile module
// Provides device-specific data abstraction for multiple keyboard models

pub mod builtin;
pub mod json;
pub mod registry;
pub mod traits;
pub mod types;

pub use builtin::{M1_V5_HE_KEY_NAMES, M1_V5_HE_MATRIX_DEFAULTS, M1V5HeProfile};
pub use json::{JsonProfile, JsonProfileWrapper, LoadError};
pub use registry::{ProfileRegistry, profile_registry};
pub use traits::{DeviceProfile, DeviceProfileExt};
pub use types::{DeviceFeatures, FnSysLayer, RangeConfig, TravelSettings};
