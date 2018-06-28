//! Packeter is a wrapper around libc for raw socket manipulation.

#![deny(missing_docs)]
#![feature(libc)]
extern crate libc;
use libc::{c_int, c_void, sockaddr_storage, socket, AF_PACKET};

use std::mem;
use std::io;

static SOCK_RAW: c_int = 3;
static IPPROTO_ICMP: c_int = 1;

struct RawSocket {
    handle: i32,
}


impl RawSocket {
    /// Create a new raw socket.
    fn new() -> io::Result<Self> {
        let handle = unsafe { socket(AF_PACKET, SOCK_RAW, libc::ETH_P_ALL.to_be() as i32) };
        match handle {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok ( RawSocket { handle }),
        }
    }
}


impl io::Read for RawSocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut addr: libc::sockaddr_ll = unsafe { mem::zeroed() };
        let mut addr_buf_sz: libc::socklen_t = mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;
        let bytes_read = unsafe {
            let addr_ptr = mem::transmute::<*mut libc::sockaddr_ll, *mut libc::sockaddr>(&mut addr);
            libc::recvfrom(
                self.handle,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                0,
                addr_ptr as *mut libc::sockaddr,
                &mut addr_buf_sz,
                )
        };

        match bytes_read {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(bytes_read as usize),
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
        use std::io::Read;
        let mut bytes = [0;10];
        let mut sock = RawSocket::new().expect("Create socket failed");
        let result = sock.read(&mut bytes).unwrap();
        println!("{:#?}", bytes);
    }
}
