/// Every ES operation has a set of options
pub type Options<'a> = Vec<(&'a str, String)>;

/// Adds a function to an operation to add specific options to that operations
/// builder interface.
macro_rules! add_option {
    ($n:ident, $e:expr) => (
        pub fn $n<T: ToString>(&'a mut self, val: &T) -> &'a mut Self {
            self.options.push(($e, val.to_string()));
            self
        }
    )
}
