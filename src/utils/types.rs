/// A wrapper around a floating-point number that provides additional functionality.
///
/// A wrapper around a floating-point number that provides additional functionality.
/// When the value is NaN, it is considered "disabled" or "turned off". This provides
/// an easy way to toggle settings without needing a separate boolean flag.
///
/// For example, instead of having both a value and an enabled flag:
/// 
/// struct Setting {
///     value: f32,
///     enabled: bool,
/// }
/// 
/// You can simply use `SmartFloat` and set it to NaN when disabled:
/// 
/// struct Setting {
///     value: SmartFloat<f32>,
/// }
/// 
use num_traits::Float;

pub struct SmartFloat<T: Float>(T);

/// Implements the functionality for the `SmartFloat` struct.
///
/// This impl block provides the implementation details for the `SmartFloat` struct,
/// which is a wrapper around a floating-point number that provides additional
/// functionality, such as checking if the value is "enabled" (i.e. not NaN).
impl<T: Float> SmartFloat<T> {
    /// Creates a new `SmartFloat` instance with the given value.
    pub fn new(value: T) -> Self {
        Self(value)
    }
    
    /// Checks if the `SmartFloat` is enabled (i.e. the value is not NaN).
    ///
    /// This method checks if the value stored in the `SmartFloat` is not NaN. This provides
    /// a convenient way to check if the `SmartFloat` is "enabled" or "turned on".
    pub fn is_enabled(&self) -> bool {
        self.0 == self.0 // Works for both f32 and f64 (NAN check)
    }

    /// Returns the underlying value stored in the `SmartFloat`.
    ///
    /// This method provides access to the raw value stored in the `SmartFloat` instance.
    pub fn get(&self) -> T {
        self.0
    }
}