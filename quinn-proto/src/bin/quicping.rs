use std::{cmp::Ordering, io, ops::Range, str};
use std::net::SocketAddr;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;
use quinn_proto::pingo::quicping;


pub fn main() {

    // let server_addr: SocketAddr = "127.0.0.1:1033".parse().unwrap();
    // Mango Validator mainnet
    let server_addr: SocketAddr = "202.8.9.108:8009".parse().unwrap();
    // testnet random https://www.validators.app/validators/9cZua5prTSEfednQQc9RkEPpbKDCh1AwnTzv3hE1eq3i?locale=en&network=testnet
    let server_addr: SocketAddr = "204.16.241.212:8009".parse().unwrap();

    let ping_result = quicping(server_addr, None);
    println!("quic-ping to {}: {}", server_addr, ping_result);


}




