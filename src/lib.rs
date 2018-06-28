//! Packeter is a wrapper around libc for raw socket manipulation.

#![deny(missing_docs)]
#![feature(libc)]
extern crate libc;
use libc::{c_int, c_void, sockaddr_storage, socket, PF_PACKET};

use std::mem;
use std::io;

const SOCK_RAW: c_int = 3;
const IPPROTO_ICMP: c_int = 1;
const IFNAMESIZE: usize = 16;
const IFREQUNIONSIZE: usize = 24;
const SIOCGIFINDEX: libc::c_ulong = 0x00008933;


struct RawSocket {
    handle: i32,
}

#[repr(C)]
struct IfReqUnion {
    data: [u8; IFREQUNIONSIZE],
}

impl Default for IfReqUnion {
    fn default() -> IfReqUnion {
        IfReqUnion { data: [0; IFREQUNIONSIZE] }
    }
}

impl IfReqUnion {
    fn as_sockaddr(&self) -> libc::sockaddr {
        let mut addr = libc::sockaddr {
            sa_family: u16::from_be((self.data[0] as u16) << 8 | (self.data[1] as u16)),
            sa_data: [0; 14],
        };

        for (i, b) in self.data[2..16].iter().enumerate() {
            addr.sa_data[i] = *b as i8;
        }

        addr
    }

    fn as_int(&self) -> libc::c_int {
        libc::c_int::from_be((self.data[0] as libc::c_int) << 24|
                             (self.data[1] as libc::c_int) << 16|
                             (self.data[2] as libc::c_int) << 8|
                             (self.data[3] as libc::c_int))
    }

    fn as_short(&self) -> libc::c_short {
        libc::c_short::from_be((self.data[0] as libc::c_short) << 8 |
                               (self.data[1] as libc::c_short))
    }
}

#[repr(C)]
struct IfReq {
    ifr_name: [libc::c_char; IFNAMESIZE],
    union: IfReqUnion,
}

impl Default for IfReq {
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
            return Err(io::Error::new(io::ErrorKind::Other, "Interface name too long."));
        }

        for (a, c) in if_req.ifr_name.iter_mut().zip(if_name.bytes()) {
            *a = c as i8;
        }

        Ok(if_req)
    }

    pub fn ifr_hwaddr(&self) -> libc::sockaddr {
        self.union.as_sockaddr()
    }

    pub fn ifr_broadaddr(&self) -> libc::sockaddr {
        self.union.as_sockaddr()
    }


    pub fn ifr_ifindex(&self) -> libc::c_int {
        self.union.as_int()
    }

    pub fn ifr_media(&self) -> libc::c_int {
        self.union.as_int()
    }

    pub fn ifr_flags(&self) -> libc::c_short {
        self.union.as_short()
    }
}


impl RawSocket {
    /// Create a new raw socket.
    fn new(interface: &str) -> io::Result<Self> {
        let handle = unsafe { socket(PF_PACKET, SOCK_RAW, libc::ETH_P_ALL.to_be() as i32) };

        if handle == -1 {
            return Err(io::Error::last_os_error());
        }

        let sock = RawSocket { handle };
        println!("sock created");

        let if_req = IfReq::with_if_name(interface)?;
        let mut req: Box<IfReq> = Box::new(if_req);

        if unsafe { libc::ioctl(sock.handle, SIOCGIFINDEX, &mut *req) } == -1 {
            return Err(io::Error::last_os_error());
        }

        println!("ioctl run");

        let mut sll : libc::sockaddr_ll = unsafe {mem::zeroed()};


        sll.sll_family = libc::AF_PACKET as u16;
        sll.sll_ifindex = req.ifr_ifindex();
        sll.sll_protocol = libc::ETH_P_ALL.to_be() as u16;
        println!("{}", mem::size_of::<libc::sockaddr_ll>());

        unsafe {
            let addr_ptr = mem::transmute::<*mut libc::sockaddr_ll, *mut libc::sockaddr>(&mut sll);
            if libc::bind(
                handle, 
                addr_ptr as *mut libc::sockaddr,
                mem::size_of::<libc::sockaddr_ll>() as u32) == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        println!("bind run");

        Ok(sock)
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
        let sock = RawSocket::new("eno1").expect("Create socket failed");
    }

    #[test]
    fn read_from_socket() {
        use super::RawSocket;
        use std::io::Read;
        let mut bytes = [0;10];
        let mut sock = RawSocket::new("eno1").expect("Create socket failed");
        let result = sock.read(&mut bytes).unwrap();
        println!("{:#?}", bytes);
    }
}
