use std::io;

pub trait FromBytes {
    /// Read from bytes.
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> where Self: Sized;
}
