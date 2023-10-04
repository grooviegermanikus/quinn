use std::error::Error;
use std::net::SocketAddrV4;
use std::str::FromStr;
use anyhow::anyhow;
use clap::Parser;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use crate::common::make_server_endpoint;

mod common;

use quinn::VarInt;


///
/// # Running
/// ```
///  cargo run --example echo_server -- -p 5002
/// ```
///

#[derive(Parser, Debug)]
#[clap(name = "echo_server")]
struct Opt {
    #[clap(short = 'p', long = "listen-port", default_value = "5000")]
    listen_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();


    let Opt { listen_port } = Opt::parse();


    let server_addr = SocketAddrV4::new("127.0.0.1".parse().unwrap(), listen_port);
    let (endpoint, _) = make_server_endpoint(server_addr.into())?;

    info!("Starting echo server on {}", server_addr);
    let endpoint_copy = endpoint.clone();

    while let Some(conn) = endpoint_copy.accept().await {
        debug!("connection incoming");
        let fut = handle_connection(conn);
        tokio::spawn(async move {
            if let Err(e) = fut.await {
                error!("connection failed: {reason}", reason = e.to_string())
            }
        });
    }

    warn!("aborting");

    endpoint.wait_idle().await;

    Ok(())
}

async fn handle_connection(conn: quinn::Connecting) -> anyhow::Result<()> {
    let connection = conn.await?;
    warn!("Set set_max_concurrent_*_streams!");
    connection.set_max_concurrent_uni_streams(VarInt::from_u32(5));
    connection.set_max_concurrent_bi_streams(VarInt::from_u32(5));
    async {
        info!("established");

        // Each stream initiated by the client constitutes a new request.
        loop {
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

    info!("echoing {} bytes", raw_payload.len());
    send.write_all(raw_payload.as_slice()).await.unwrap();
    send.finish().await.unwrap();

    Ok(())
}
