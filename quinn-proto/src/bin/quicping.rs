use std::{cmp::Ordering, io, ops::Range, str};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;
use quinn_proto::pingo::{PingResult, quicping};


pub fn main() {
    tracing_subscriber::fmt::fmt().with_max_level(tracing::Level::DEBUG).init();

    // let server_addr: SocketAddr = "127.0.0.1:1033".parse().unwrap();
    // Mango Validator mainnet
    let server_addr: SocketAddr = "202.8.9.108:8009".parse().unwrap();
    // testnet random https://www.validators.app/validators/9cZua5prTSEfednQQc9RkEPpbKDCh1AwnTzv3hE1eq3i?locale=en&network=testnet
    // let server_addr: SocketAddr = "204.16.241.212:8009".parse().unwrap();
    // let server_addr: SocketAddr = "93.115.25.181:12003".parse().unwrap();
    let server_addr: SocketAddr = "93.115.25.181:22".parse().unwrap();

    let ping_result = tcp_ping(server_addr, Duration::from_millis(500));
    println!("tcp-ping to {}: {}", server_addr, ping_result);

    // let ping_result = quicping(server_addr, None);

    // println!("quic-ping to {}: {}", server_addr, ping_result);


}

fn tcp_ping(addr: SocketAddr, timeout: Duration) -> PingResult {
    let started_at = Instant::now();
    return match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_stream) => {
            let duration = started_at.elapsed();
            println!(
                "Probing {}/tcp - Port is open - time={:?}",
                addr, duration
            );
            PingResult::Success(duration)
        }
        Err(_) => {
            let duration = timeout.as_micros() as f64 / 1000.0;
            println!(
                "Probing {}/tcp - No response - time={:.4}ms",
                addr, duration
            );
            PingResult::Timeout
        }
    }
}




