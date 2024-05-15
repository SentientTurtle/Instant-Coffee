//! Specialized interop for Java types/features that do not cleanly map onto rust

/// Struct representing Java `char` type. 16-bits numerical value for UTF-16 code units.
///
/// Unlike Rust's char, permits all u16 values (0..=0xFFFF), and may be directly created from u16
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct JavaChar(pub u16);

impl JavaChar {
    /// Attempt to convert a rust `char` into a JavaChar
    ///
    /// This will fail if char is > U+FFFF, and succeed otherwise
    pub fn from_char(char: char) -> Option<JavaChar> {
        if (char as u32) <= (u16::MAX as u32) {
            Some(JavaChar(char as u16))
        } else {
            None
        }
    }

    /// Attempt to convert JavaChar into rust `char`
    ///
    /// This will fail if self is a surrogate pair value
    pub fn into_char(self) -> Option<char> {
        char::try_from(self.0 as u32).ok()
    }
}