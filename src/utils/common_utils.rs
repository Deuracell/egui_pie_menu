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



#[derive(Debug, Clone)]
pub struct BoundedVec<T> {
    items: Vec<T>,  // Private to prevent direct modification
    max_size: usize, // Maximum number of elements allowed
}

impl<T> BoundedVec<T> {
    /// Creates a new `BoundedVec`, enforcing at least one item
    pub fn new(mut items: Vec<T>, max_size: usize) -> Result<Self, &'static str> {
        if items.is_empty() {
            return Err("BoundedVec must have at least one element");
        }
        items.truncate(max_size); // Trim excess if over max
        Ok(Self { items, max_size })
    }

    /// Returns a reference to stored items
    pub fn get(&self) -> &[T] {
        &self.items
    }

    /// Adds an item only if it doesn't exceed the max size
    pub fn push(&mut self, item: T) -> Result<(), &'static str> {
        if self.items.len() < self.max_size {
            self.items.push(item);
            Ok(())
        } else {
            Err("Exceeded max size")
        }
    }

    /// Removes the last item, but prevents removing the last remaining one
    pub fn pop(&mut self) -> Result<T, &'static str> {
        if self.items.len() > 1 {
            Ok(self.items.pop().unwrap())
        } else {
            Err("Cannot remove the last element")
        }
    }
}
