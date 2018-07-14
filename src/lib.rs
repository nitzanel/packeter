//! Packeter is a wrapper around libc for raw socket manipulation.
#![deny(missing_docs)]
#![feature(libc)]
extern crate libc;

mod low_level_interfaces {
    use libc;
    use libc::{
        c_char, c_int, c_short, c_ulong, c_void, sockaddr, sockaddr_ll, sockaddr_storage, socket,
        socklen_t, AF_PACKET,
    };

    use std::io;
    use std::mem;
    /// Size of the interface name.
    const IFNAMESIZE: usize = 16;
    /// Size of the interface request union.
    const IFREQUNIONSIZE: usize = 24;
    /// Value of SIOCGIFINDEX to get an interface id by name.
    const SIOCGIFINDEX: c_ulong = 0x00008933;

    /// The union part of the IfReq struct.
    /// This part is implamented so we can use data as we want
    #[repr(C)]
    struct IfReqUnion {
        data: [u8; IFREQUNIONSIZE],
    }


    impl Default for IfReqUnion {
        /// Creates an empty IfReqUnion
        fn default() -> IfReqUnion {
            IfReqUnion {
                data: [0; IFREQUNIONSIZE],
            }
        }
    }

    /// The following functions allows us to get data as the format we want, without unsafe
    /// casting.
    impl IfReqUnion {
        /// Get the IfReqUnion part as a sockaddr
        fn as_sockaddr(&self) -> sockaddr {
            let mut addr = sockaddr {
                sa_family: u16::from_be((self.data[0] as u16) << 8 | (self.data[1] as u16)),
                sa_data: [0; 14],
            };

            for (i, b) in self.data[2..16].iter().enumerate() {
                addr.sa_data[i] = *b as i8;
            }

            addr
        }

        /// Get the IfReqUnion part as an int32
        fn as_int(&self) -> c_int {
            c_int::from_be(
                (self.data[0] as c_int) << 24
                    | (self.data[1] as c_int) << 16
                    | (self.data[2] as c_int) << 8
                    | (self.data[3] as c_int),
            )
        }

        /// Get the IfReqUnion part as a int16
        fn as_short(&self) -> c_short {
            c_short::from_be((self.data[0] as c_short) << 8 | (self.data[1] as c_short))
        }
    }

    /// The IfReq struct from libc
    #[repr(C)]
    pub struct IfReq {
        ifr_name: [c_char; IFNAMESIZE],
        union: IfReqUnion,
    }

    impl Default for IfReq {
        /// Creates an empty IfReq
        fn default() -> IfReq {
            IfReq {
                ifr_name: [0; IFNAMESIZE],
                union: IfReqUnion::default(),
            }
        }
    }

    impl IfReq {
        /// Create an interface request struct with the interace name set
        pub fn with_if_name(if_name: &str) -> io::Result<IfReq> {
            let mut if_req = IfReq::default();

            if if_name.len() >= if_req.ifr_name.len() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Interface name too long.",
                ));
            }

            for (a, c) in if_req.ifr_name.iter_mut().zip(if_name.bytes()) {
                *a = c as i8;
            }

            Ok(if_req)
        }

        /// Get the hardware address
        pub fn ifr_hwaddr(&self) -> sockaddr {
            self.union.as_sockaddr()
        }

        pub fn ifr_broadaddr(&self) -> sockaddr {
            self.union.as_sockaddr()
        }

        pub fn ifr_ifindex(&self) -> c_int {
            self.union.as_int()
        }

        pub fn ifr_media(&self) -> c_int {
            self.union.as_int()
        }

        pub fn ifr_flags(&self) -> c_short {
            self.union.as_short()
        }
    }

}

/// Raw module, for handling raw sockets
mod raw_socket {
    use libc::{
        c_int, c_void, close, recvfrom, sockaddr, sockaddr_ll, socket, socklen_t, AF_PACKET, setsockopt, SO_BINDTODEVICE, SOL_SOCKET, bind, sendto,
    };
    use std::io;
    use std::io::{ Read, Write };
    use std::mem;

    use low_level_interfaces::IfReq;

    // TODO: should it be c_int or i32?
    const SOCK_RAW: c_int = 3;
    const ETH_P_ARP: u16 = 0x0003;

    /// Wrapper around a file descriptor recieved from creating a socket.
    pub struct RawSocket {
        pub handle: i32,
        interface: IfReq,
    }

    impl RawSocket {
        /// Create a new raw socket.
        pub fn new(interface: String) -> io::Result<Self> {
            let interface = IfReq::with_if_name(interface.as_str())?;
            let handle = unsafe { socket(AF_PACKET, SOCK_RAW, ETH_P_ARP.to_be() as i32) };

            match handle {
                -1 => Err(io::Error::last_os_error()),
                _ => Ok(RawSocket { handle, interface }),
            }
        }

        /// Binds the socket to the given interface.
        /// Interface name must not be longer then `IFNAMESIZE`
        pub fn bind(&self, interface: &str) -> io::Result<()> {
            unsafe {
                let ifreq = IfReq::with_if_name(interface).expect("Failed to create IfReq");
                match bind(self.handle, &ifreq.ifr_hwaddr() as *const sockaddr, mem::size_of::<sockaddr_ll>() as socklen_t) {
                    -1 => Err(io::Error::last_os_error()),
                    _ => Ok(()),
                }
            }
        }

        fn send_bytes(&self, bytes: &[u8]) -> io::Result<usize> {
            let length =  unsafe {
                sendto(self.handle, bytes.as_ptr() as *const c_void , bytes.len(), 0, &self.interface.ifr_hwaddr(), mem::size_of::<sockaddr_ll>() as socklen_t)
            };

            match length {
                -1 => Err(io::Error::last_os_error()),
                _ => Ok(length as usize),
            }
        }


        /// Recieve a single packet from the socket.
        /// This method blocks untill the read is completed.
        fn recvfrom(&self, buf: &mut [u8]) -> io::Result<usize> {
            let mut sender_addr: sockaddr_ll = unsafe { mem::zeroed() };
            let mut addr_buf_sz: socklen_t = mem::size_of::<sockaddr_ll>() as socklen_t;
            unsafe {
                // We need to cast the sender address (sockaddr_ll) into a sockaddr.
                let addr_ptr = mem::transmute::<*mut sockaddr_ll, *mut sockaddr>(&mut sender_addr);
                match recvfrom(
                    self.handle,                     // file descriptor
                    buf.as_mut_ptr() as *mut c_void, // pointer to buffer for frame content
                    buf.len(),                       // frame content buffer length
                    0,                               // flags
                    addr_ptr as *mut sockaddr,       // pointer to buffer for sender address
                    &mut addr_buf_sz,                // sender address buffer length
                ) {
                    -1 => Err(io::Error::last_os_error()),
                    len => Ok(len as usize),
                }
            }
        }

    }

    impl Drop for RawSocket {
        /// Close the socket handle (fd)
        fn drop(&mut self) {
            unsafe {
                close(self.handle);
            }
        }
    }

    impl Read for RawSocket {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.recvfrom(buf)
        }
    }

    impl Write for RawSocket {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.send_bytes(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            unimplemented!();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn create_raw_socket() {
        use super::raw_socket::RawSocket;
        let sock = RawSocket::new("wlp2s0".to_string()).expect("Create socket failed");
    }

    #[test]
    fn read_from_socket() {
        use super::raw_socket::RawSocket;
        use libc;
        use std::io::Read;
        let mut packet_buf: [u8; 1024] = [0; 1024];
        let mut sock = RawSocket::new("wlp2s0".to_string()).expect("Create socket failed");
        sock.read(&mut packet_buf)
            .expect("Failed to recvfrom socket");
    }

    #[test]
    fn write_to_socket() {
        use super::raw_socket::RawSocket;
        use libc;
        use std::io::{Read, Write};
        let bytes = "message".as_bytes();
        let mut sock = RawSocket::new("wlp2s0".to_string()).expect("Create socket failed");
        //sock.bind("wlp2s0").expect("Failed to bind to interface wlp2s0");
        sock.write(bytes).expect("Failed to write to socket");

    }
}
