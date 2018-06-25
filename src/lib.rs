#![feature(libc)]
extern crate libc;
use libc::{c_int, c_void, socket, AF_PACKET, sockaddr_storage};

static SOCK_RAW: c_int = 3;
static IPPROTO_ICMP: c_int = 1;

fn create_raw_socket() -> i32 {
    let handle = unsafe {
        socket(AF_PACKET, SOCK_RAW, libc::ETH_P_ALL.to_be() as i32)
    };
    println!("handle: {}", handle);
    return handle;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use super::*;
        let handle = create_raw_socket();
        assert_eq!(handle, 3);
    }
}
