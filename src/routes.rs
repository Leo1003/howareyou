use std::convert::Infallible;
use std::net::SocketAddr;

use anyhow::Result as AnyResult;
use futures::{FutureExt, StreamExt};
use warp::hyper::{HeaderMap, StatusCode};
use warp::reply::with_status;
use warp::ws::Ws;
use warp::Reply;

pub async fn root(remote_addr: Option<SocketAddr>, headers: HeaderMap) -> Result<impl Reply, Infallible> {
    Ok(String::new())
}

pub async fn api(remote_addr: Option<SocketAddr>, headers: HeaderMap) -> Result<impl Reply, Infallible> {
    Ok(String::new())
}

pub fn health() -> impl Reply {
    with_status("ok", StatusCode::OK)
}

pub fn ws(ws: Ws) -> impl Reply {
    ws.on_upgrade(|ws| {
        // Pipe the message back
        let (tx, rx) = ws.split();
        rx.forward(tx).map(|result| {
            if let Err(e) = result {
                error!("websocket error: {:?}", e);
            }
        })
    })
}
