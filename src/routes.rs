use futures::{future, FutureExt, StreamExt, TryStreamExt};
use std::convert::Infallible;
use warp::hyper::StatusCode;
use warp::reply::{json, with_status};
use warp::ws::Ws;
use warp::Reply;

use crate::data::ClientInfo;

pub async fn root(client_info: ClientInfo) -> Result<impl Reply, Infallible> {
    Ok(format!("{}", &client_info))
}

pub async fn api(client_info: ClientInfo) -> Result<impl Reply, Infallible> {
    Ok(json(&client_info))
}

pub fn health() -> impl Reply {
    with_status("ok", StatusCode::OK)
}

pub fn ws(ws: Ws) -> impl Reply {
    ws.on_upgrade(|ws| {
        info!("New Websocket connection created!");
        // Pipe the message back
        let (tx, rx) = ws.split();
        rx.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
            .forward(tx)
            .map(|result| {
                if let Err(e) = result {
                    error!("websocket error: {:?}", e);
                }
                debug!("Websocket connection closed");
            })
    })
}
