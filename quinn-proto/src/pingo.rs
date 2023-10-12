use std::{cmp::Ordering, io, ops::Range, str};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use rand::RngCore;
use thiserror::Error;

use crate::{coding::{self, BufExt, BufMutExt}, crypto, ConnectionId, RandomConnectionIdGenerator, ConnectionIdGenerator};
use crate::{crypto::rustls::initial_keys, Side};
use rustls::quic::{HeaderProtectionKey, Version};
use tracing::{debug, trace};
use crate::crypto::{HeaderKey, KeyPair, Keys};
use crate::packet::{Header, LongType, PacketNumber, PartialDecode};

const SUPPORTED_VERSION: u32 = 0xbabababa;

pub fn quicping(server_addr: SocketAddr, recv_timeout: Option<Duration>) -> Duration {

    let mut cid_generator = RandomConnectionIdGenerator::new(12);

    let dcid = cid_generator.generate_cid();

    let (buf, client_keys) = build_packet(dcid);

    let socket = UdpSocket::bind("0.0.0.0:0").expect("couldn't bind to address");
    // Mango Validator mainnet
    // let server_addr: SocketAddr = "202.8.9.108:8009".parse().unwrap();
    // testnet random https://www.validators.app/validators/9cZua5prTSEfednQQc9RkEPpbKDCh1AwnTzv3hE1eq3i?locale=en&network=testnet
    // let server_addr: SocketAddr = "81.16.237.158:8010".parse().unwrap();

    let start_ts = Instant::now();
    socket.send_to(&buf, server_addr).expect("couldn't send data");

    let mut recv_buf = [0; 1232];
    socket.set_read_timeout(recv_timeout).unwrap();
    let recv_result = socket.recv(&mut recv_buf);
    let elapsed = start_ts.elapsed();
    match recv_result {
        Ok(received) => debug!("received {received} bytes {:?}", &buf[..received]),
        // on timeout: Os { code: 35, kind: WouldBlock, message: "Resource temporarily unavailable" }
        Err(e) => debug!("recv function failed: {e:?}"),
    }

    trace!("response:");
    for byte in &recv_buf {
        trace!("{:02x}", byte);
    }

    let supported_versions = vec![SUPPORTED_VERSION];

    let decode = PartialDecode::new(buf.as_slice().into(), 0, &supported_versions, false)
        .unwrap()
        .0;
    let mut packet = decode.finish(Some(&*client_keys.header.remote)).unwrap();

    match packet.header {
        Header::Long { .. } => {
            trace!("packet: {}", packet.header.dst_cid());
            assert_eq!(&dcid, packet.header.dst_cid());
        }
        _ => { trace!("we want long header"); }
    }


    // to testnet:
    // ICMP 57.503 ms
    // QUIC 63967us

    // to localhost
    // 124us
    // 176us
    debug!("quic-ping to {} to {:.3}ms", server_addr, elapsed.as_secs_f64() * 1000.0);

    elapsed
}

fn build_packet(dcid: ConnectionId) -> (Vec<u8>, Keys) {
    let client_keys = initial_keys(Version::V1, &dcid, Side::Client);
    let mut buf = Vec::new();
    let header = Header::Long {
        ty: LongType::Handshake,
        number: PacketNumber::U8(2), // pn = packet number
        src_cid: ConnectionId::new(&[]),
        dst_cid: dcid,
        version: SUPPORTED_VERSION,
    };
    let encode = header.encode(&mut buf);
    let header_len = buf.len();
    // buf.resize(header_len + 16 + client.packet.local.tag_len(), 0);
    // pad to 1200 according to https://www.rfc-editor.org/rfc/rfc9000.html#name-initial-packet
    buf.resize(1200, 0);
    rand::thread_rng().fill_bytes(&mut buf[header_len..]);
    encode.finish(
        &mut buf,
        &*client_keys.header.local,
        Some((0, &*client_keys.packet.local)),
    );

    trace!("packet");
    for byte in &buf {
        trace!("{:02x}", byte);
    }
    assert_eq!(buf.len(), 1200);
    (buf, client_keys)
}
