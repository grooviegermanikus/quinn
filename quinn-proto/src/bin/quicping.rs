use std::{cmp::Ordering, io, ops::Range, str};
use std::net::SocketAddr;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;
use quinn_proto::pingo::quicping;


pub fn main() {

    let server_addr: SocketAddr = "127.0.0.1:1033".parse().unwrap();
    // Mango Validator mainnet
    // let server_addr: SocketAddr = "202.8.9.108:8009".parse().unwrap();
    // testnet random https://www.validators.app/validators/9cZua5prTSEfednQQc9RkEPpbKDCh1AwnTzv3hE1eq3i?locale=en&network=testnet
    // let server_addr: SocketAddr = "81.16.237.158:8010".parse().unwrap();


    let duration = quicping(server_addr, None);
    println!("quic-ping to {}: {:.3}ms", server_addr, duration.as_secs_f64() * 1000.0);


}




