//! Packeter is a wrapper around libc for raw socket manipulation.

#![deny(missing_docs)]
#![feature(libc)]
extern crate libc;
use libc::{c_int, c_void, sockaddr_storage, socket, AF_PACKET};

use std::io;

static SOCK_RAW: c_int = 3;
static IPPROTO_ICMP: c_int = 1;

struct RawSocket {
    handle: i32,
}

// TODO: Create custom error and return a result with this error type.

impl RawSocket {
    /// Create a new raw socket.
    fn new() -> io::Result<Self> {
        let handle = unsafe { socket(AF_PACKET, SOCK_RAW, libc::ETH_P_ALL.to_be() as i32) };
        match handle {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(RawSocket { handle }),
            }
        }

    fn read_bytes(&self, bytes_to_read: usize) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(bytes_to_read);
        // Vector is already of that capacity
        unsafe {
            bytes.set_len(bytes_to_read);
        };
        // read the bytes
        let bytes_read = unsafe {
            libc::read(
                self.handle,
                bytes.as_mut_slice().as_mut_ptr() as *mut c_void,
                bytes_to_read,
            )
        };

        match bytes_read {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(bytes),
        }
    }
}

impl Drop for RawSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn create_raw_socket() {
        use super::RawSocket;
        RawSocket::new().expect("Create socket failed");
    }

    #[test]
    fn read_from_socket() {
        use super::RawSocket;
        let sock = RawSocket::new().expect("Create socket failed");
        sock.read_bytes(10).unwrap();
    }
}
