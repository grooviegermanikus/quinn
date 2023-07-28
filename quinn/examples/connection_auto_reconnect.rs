//! This example intends to use the smallest amount of code to make a simple QUIC connection.
//!
//! Checkout the `README.md` for guidance.

mod common;

use std::cell::{Cell, Ref, RefCell};
use std::fmt;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use rustls::ClientConfig;
use tracing::{debug, error, info, warn};
use tracing::field::debug;
use common::{make_client_endpoint, make_server_endpoint};
use quinn::{Connecting, Connection, Endpoint};
use quinn::VarInt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    configure_logging();

    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let (endpoint, server_cert) = make_server_endpoint(server_addr)?;
    // accept a single connection
    let endpoint2 = endpoint.clone();
    let join_handle = tokio::spawn(async move {
        // let incoming_conn = endpoint2.accept().await.unwrap();

        let mut count = 0;
        while let Some(conn) = endpoint2.accept().await {
            info!("connection incoming {}", count);
            count += 1;
            let fut = handle_connection(conn);
            tokio::spawn(async move {
                if let Err(e) = fut.await {
                    error!("connection failed: {reason}", reason = e.to_string())
                }
            });
        }

        info!("shutting down");
    });

    let endpoint = make_client_endpoint("0.0.0.0:0".parse().unwrap(), &[&server_cert])?;
    // connect to server
    // let connection = endpoint
    //     .connect(server_addr, "localhost")
    //     .unwrap()
    //     .await
    //     .unwrap();
    // println!("[client] connected: addr={}", connection.remote_address());

    let auto_connect = AutoReconnect::new(endpoint, server_addr);

    let mut ticker = tokio::time::interval(Duration::from_millis(1000));
    loop {
        let result = roundtrip(&auto_connect).await;
        if result.is_err() {
            warn!("roundtrip failed: {:?}", result);
        }


        ticker.tick().await;
    }



    // Waiting for a stream will complete with an error when the server closes the connection
    // let _ = auto_connect.refresh().accept_uni().await;

    // Give the server has a chance to clean up
    // endpoint.wait_idle().await;

    Ok(())
}

async fn roundtrip(auto_connect: &AutoReconnect) -> anyhow::Result<()> {
    println!(">>");
    let (mut send_stream, recv_stream) = auto_connect.refresh().await.open_bi().await?;
    send_stream.write_all("HELLO".as_bytes()).await?;
    send_stream.finish().await;

    let answer = recv_stream.read_to_end(64 * 1024).await?;
    println!("answer: {:?}", answer);

    Ok(())
}


async fn handle_connection(conn: quinn::Connecting) -> anyhow::Result<()> {
    let connection = conn.await?;
    async {
        info!("established");

        let mut count = 0;
        // Each stream initiated by the client constitutes a new request.
        loop {

            if count == 5 {
                connection.close(VarInt::from_u32(99), b"server closed");
            }
            count += 1;

            let stream = connection.accept_bi().await;
            let stream = match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("connection closed");
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
                Ok(s) => s,
            };
            let fut = handle_request(stream);
            tokio::spawn(
                async move {
                    if let Err(e) = fut.await {
                        error!("failed: {reason}", reason = e.to_string());
                    }
                }
            );
        }
    }
        .await?;
    Ok(())
}

async fn handle_request(
    (mut send, recv): (quinn::SendStream, quinn::RecvStream),
) -> anyhow::Result<()> {
    let raw_payload = recv
        .read_to_end(64 * 1024)
        .await
        .map_err(|e| anyhow!("failed reading request: {}", e))?;

    // TODO respond

    let str = String::from_utf8_lossy(raw_payload.as_slice());
    println!("request: {:?}", str);
    send.write_all(raw_payload.as_slice()).await?;
    send.finish().await?;

    Ok(())
}

struct AutoReconnect {
    endpoint: Endpoint,
    current: RefCell<Option<Connection>>,
    target_address: SocketAddr,
}

impl AutoReconnect {
    pub fn new(endpoint: Endpoint, target_address: SocketAddr) -> Self {
        Self {
            endpoint,
            current: RefCell::new(None),
            target_address,
        }
    }

    pub async fn refresh(&self) -> Connection {
        let mut foo = self.current.borrow_mut();
        match &*foo {
            Some(current) => {

                if current.close_reason().is_some() {
                    info!("Connection is closed for reason: {:?}", current.close_reason());
                    // TODO log

                    let new_connection = self.create_connection().await;
                    let old_conn = foo.replace(new_connection.clone());
                    debug!("Replace closed connection {} with {}",
                        old_conn.map(|c| c.stable_id().to_string()).unwrap_or("none".to_string()),
                        new_connection.stable_id());
                    // TODO log old vs new stable_id

                    return new_connection.clone();
                } else {
                    debug!("Reuse connection {}", current.stable_id());
                    return current.clone();
                }

            }
            None => {
                let new_connection = self.create_connection().await;

                let old_conn = foo.replace(new_connection.clone());
                assert!(old_conn.is_none(), "old connection should be None");
                // let old_conn = foo.replace(Some(new_connection.clone()));
                // TODO log old vs new stable_id
                debug!("Create initial connection {}", new_connection.stable_id());

                return new_connection.clone();
            }
        }
    }

    async fn create_connection(&self) -> Connection {
        let connection =
            self.endpoint.connect(self.target_address, "localhost").expect("handshake");

        connection.await.expect("connection")
    }
}

impl fmt::Display for AutoReconnect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Connection to {}",
           self.target_address,
        )
    }
}

fn configure_logging() {
    tracing_subscriber::fmt::fmt()
        // .with_env_filter( "debug,quinn=trace,quinn_proto=trace")
        .with_env_filter( "debug,quinn=debug,quinn_proto=debug")
        .init();
}

