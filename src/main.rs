#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

use std::{
    convert::Infallible,
    env,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use anyhow::{Context, Result as AnyhowResult};
use dotenv::dotenv;
use tokio::runtime::Builder as RuntimeBuilder;
use warp::{Filter, Rejection};

mod data;
mod routes;

fn main() -> AnyhowResult<()> {
    dotenv().ok();
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::init();

    let rt_type =
        env::var("RUNTIME_TYPE").unwrap_or_else(|_| "multithread".to_owned());

    let rt = match rt_type.as_str() {
        "multithread" | "multi_thread" => {
            let mut builder = RuntimeBuilder::new_multi_thread();

            let worker_cnt = env::var("RUNTIME_WORKERS")
                .ok()
                .map(|var| var.parse::<usize>())
                .transpose()
                .with_context(|| "invalid RUNTIME_WORKERS variable")?;

            if let Some(worker_cnt) = worker_cnt {
                builder.worker_threads(worker_cnt);
            }

            builder.enable_all().build()?
        }
        "singlethread" | "single_thread" | "current_thread" => {
            RuntimeBuilder::new_current_thread().enable_all().build()?
        }
        _ => bail!("Invalid RUNTIME_TYPE variable"),
    };

    rt.block_on(launch_server())
}

async fn launch_server() -> AnyhowResult<()> {
    let root = warp::path::end().and(
        data::client_info()
            .and_then(routes::root)
            .with(warp::wrap_fn(wait_wrapper)),
    );
    let api = warp::path("api").and(
        data::client_info()
            .and_then(routes::api)
            .with(warp::wrap_fn(wait_wrapper)),
    );
    let ws = warp::path("ws").and(warp::ws()).map(routes::ws);
    let health = warp::path("health")
        .and(warp::get().or(warp::head()).unify())
        .map(routes::health);

    let router = root // GET /
        .or(api) // GET /api
        .or(health) // GET /health
        .or(ws) // GET /ws
        .with(warp::log("howareyou"));

    let addr = env::var("APP_ADDRESS")
        .ok()
        .map(|var| var.parse::<IpAddr>())
        .transpose()
        .with_context(|| "invalid APP_ADDRESS variable")?
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let port = env::var("APP_PORT")
        .ok()
        .map(|var| var.parse::<u16>())
        .transpose()
        .with_context(|| "invalid APP_PORT variable")?
        .unwrap_or(8080);

    info!("Listening on {}:{}", addr, port);
    warp::serve(router).run((addr, port)).await;
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WaitTime {
    #[serde(with = "humantime_serde")]
    wait: Duration,
}

/// Process wait query
fn wait_wrapper<F, T>(
    filter: F,
) -> impl Filter<Extract = (T,), Error = Rejection> + Clone + Send + Sync + 'static
where
    F: Filter<Extract = (T,), Error = Rejection>
        + Clone
        + Send
        + Sync
        + 'static,
    F::Extract: warp::Reply,
{
    let wait_filter = warp::query::<WaitTime>()
        .and_then(wait_handler)
        .untuple_one();
    wait_filter.or(warp::any()).unify().and(filter)
}

async fn wait_handler(waittime: WaitTime) -> Result<(), Infallible> {
    tokio::time::sleep(waittime.wait).await;
    Ok(())
}
