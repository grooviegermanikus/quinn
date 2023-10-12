use std::{cmp::Ordering, io, ops::Range, str};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

use crate::{coding::{self, BufExt, BufMutExt}, crypto, ConnectionId, RandomConnectionIdGenerator, ConnectionIdGenerator};
use crate::{crypto::rustls::initial_keys, Side};
use rustls::quic::Version;
use crate::packet::{Header, LongType, PacketNumber, PartialDecode};


pub fn pingo() {
    use crate::{crypto::rustls::initial_keys, Side};
    use rustls::quic::Version;

    let supported_version = 0xbabababa;

    // TODO random
    // 8 - 18 bytes
    let mut cid_generator = RandomConnectionIdGenerator::new(8);

    let dcid = cid_generator.generate_cid();
    let client = initial_keys(Version::V1, &dcid, Side::Client);
    let mut buf = Vec::new();
    let header = Header::Long {
        ty: LongType::Handshake,
        number: PacketNumber::U8(2), // pn = packet number
        src_cid: ConnectionId::new(&[]),
        dst_cid: dcid,
        version: supported_version,
    };
    let encode = header.encode(&mut buf);
    // let header_len = buf.len();
    // buf.resize(header_len + 16 + client.packet.local.tag_len(), 0);
    // pad to 1200 according to https://www.rfc-editor.org/rfc/rfc9000.html#name-initial-packet
    // TODO random content
    buf.resize(1200, 0);
    encode.finish(
        &mut buf,
        &*client.header.local,
        Some((0, &*client.packet.local)),
    );

    // TODO we should get c3
    // should be c3babababa12d71a27b19f471ace2e1a5b087003094242b210258c2e2de85341104ea3e426cc34164800449400000002
    println!("packet");
    for byte in &buf {
        print!("{:02x}", byte);
    }
    println!();
    assert_eq!(buf.len(), 1200);

    // TODO use dual-stack
    let socket = UdpSocket::bind("0.0.0.0:0").expect("couldn't bind to address");
    let server_addr: SocketAddr = "127.0.0.1:1033".parse().unwrap();
    // Mango Validator mainnet
    // let server_addr: SocketAddr = "202.8.9.108:8009".parse().unwrap();
    // testnet random https://www.validators.app/validators/9cZua5prTSEfednQQc9RkEPpbKDCh1AwnTzv3hE1eq3i?locale=en&network=testnet
    // let server_addr: SocketAddr = "81.16.237.158:8010".parse().unwrap();

    let start_ts = Instant::now();
    socket.send_to(&buf, server_addr).expect("couldn't send data");

    let mut recv_buf = [0; 1000];
    socket.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    let recv_result = socket.recv(&mut recv_buf);
    let elapsed = start_ts.elapsed();
    match recv_result {
        Ok(received) => println!("received {received} bytes {:?}", &buf[..received]),
        // on timeout: Os { code: 35, kind: WouldBlock, message: "Resource temporarily unavailable" }
        Err(e) => println!("recv function failed: {e:?}"),
    }

    println!("response:");
    for byte in &recv_buf {
        print!("{:02x}", byte);
    }
    println!();

    let supported_versions = vec![supported_version];

    let decode = PartialDecode::new(buf.as_slice().into(), 0, &supported_versions, false)
        .unwrap()
        .0;
    let mut packet = decode.finish(Some(&*client.header.remote)).unwrap();

    match packet.header {
        Header::Long { .. } => {
            println!("packet: {}", packet.header.dst_cid());
            assert_eq!(&dcid, packet.header.dst_cid());
        }
        _ => { panic!("we want long header"); }
    }


    // to testnet:
    // ICMP 57.503 ms
    // QUIC 63967us

    // to localhost
    // 124us
    // 176us
    println!("quic-ping to {} to {:.3}ms", server_addr, elapsed.as_secs_f64() * 1000.0);
}
