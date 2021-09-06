#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

use std::{convert::Infallible, time::Duration};

use anyhow::{Context, Result};
use tokio::runtime::Builder as RuntimeBuilder;
use warp::{Filter, Rejection};

mod data;
mod routes;

fn main() -> Result<()> {
    env_logger::init();

    let rt_type = dotenv::var("RUNTIME_TYPE")
        .unwrap_or_else(|_| "multithread".to_owned());

    let rt = match rt_type.as_str() {
        "multithread" | "multi_thread" => {
            let mut builder = RuntimeBuilder::new_multi_thread();

            let worker_cnt = dotenv::var("RUNTIME_WORKERS")
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

    rt.block_on(bootstrap())
}

async fn bootstrap() -> Result<()> {
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

    warp::serve(router).run(([127, 0, 0, 1], 8080)).await;
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
