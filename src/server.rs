// Imports

use crate::config::{COOLDOWN, PRUNE_TIME, RATELIMIT};
use crate::proxy;
use crate::utils;

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::time::sleep;
use warp::http::StatusCode;
use warp::reject::Reject;
use warp::{Filter, Rejection, Reply, Server};

// Ratelimit counter for ip

struct Ratelimit {
    timestamp: AtomicU64,
    requests: AtomicU32,
}

impl Ratelimit {
    // Construct new counter with current time

    fn init() -> Ratelimit {
        Ratelimit {
            timestamp: AtomicU64::new(utils::get_time()),
            requests: AtomicU32::new(1),
        }
    }
}

// Ratelimit error

#[derive(Debug)]
struct RatelimitExceeded {
    retry_after: u64,
}

impl Reject for RatelimitExceeded {}

// Create HTTP server with ratelimit map

#[allow(opaque_hidden_inferred_bound)]
pub fn create_server() -> Server<impl Filter<Extract = impl Reply> + Clone> {
    // Prune ratelimits on loop

    let ratelimits = Arc::new(DashMap::<IpAddr, Ratelimit>::new());
    tokio::spawn({
        let ratelimits = ratelimits.clone();
        async move {
            loop {
                sleep(Duration::from_secs(PRUNE_TIME)).await;
                prune_ratelimits(&ratelimits);
            }
        }
    });

    // Run server with routes

    warp::serve(build_routes(ratelimits))
}

// Build server routes

#[allow(opaque_hidden_inferred_bound)]
fn build_routes(
    ratelimits: Arc<DashMap<IpAddr, Ratelimit>>,
) -> impl Filter<Extract = impl Reply> + Clone {
    // HTTP request client

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();

    // GET request proxy

    warp::get()
        .and(warp::addr::remote().and_then(move |addr| check_ratelimit(ratelimits.clone(), addr)))
        .and(warp::path::full())
        .and(
            warp::query::raw()
                .map(Some)
                .or_else(|_| async { Ok::<_, Infallible>((None,)) }),
        )
        .then(move |_, path, query| {
            let client = client.clone();
            async {
                match proxy::get(client, path, query).await {
                    Ok(reply) => reply,
                    Err(_) => Box::new(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({
                            "message": "internal error",
                        })),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )),
                }
            }
        })
        .recover(handle_rejection)
        .with(warp::reply::with::header(
            "access-control-allow-origin",
            "*",
        ))
}

// Create rejection response

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    if let Some(ratelimit) = err.find::<RatelimitExceeded>() {
        // Ratelimit exceeded response

        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "message": "ratelimit exceeded",
                "retryAfter": ratelimit.retry_after,
            })),
            StatusCode::TOO_MANY_REQUESTS,
        ))
    } else {
        // Internal error

        Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "message": "internal error",
            })),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

// Check ratelimit of ip address

async fn check_ratelimit(
    ratelimits: Arc<DashMap<IpAddr, Ratelimit>>,
    addr: Option<SocketAddr>,
) -> Result<(), Rejection> {
    // Get or insert ip into map

    let ip = match addr {
        Some(addr) => addr.ip(),
        None => return Ok(()),
    };
    let Some(counter) = ratelimits.get(&ip) else {
        ratelimits.insert(ip, Ratelimit::init());
        return Ok(());
    };

    // Check request count in last 5 seconds

    let timestamp = counter.timestamp.load(Ordering::Acquire);
    let now = utils::get_time();

    if timestamp + 5000 < now {
        // New requests not in old time bucket

        counter.timestamp.store(now, Ordering::Release);
        counter.requests.store(1, Ordering::Release);
        return Ok(());
    } else if timestamp <= now {
        let reqs = counter.requests.load(Ordering::Acquire);
        if reqs < RATELIMIT * 5 {
            // Requests within ratelimit

            counter.requests.store(reqs + 1, Ordering::Release);
            return Ok(());
        } else {
            // Requests over ratelimit

            counter
                .timestamp
                .store(now + COOLDOWN * 1000, Ordering::Release);
            counter.requests.store(0, Ordering::Release);
            return Err(warp::reject::custom(RatelimitExceeded {
                retry_after: COOLDOWN * 1000,
            }));
        }
    }

    // Ratelimit exceeded error

    Err(warp::reject::custom(RatelimitExceeded {
        retry_after: timestamp - now,
    }))
}

// Remove unused ratelimit entries from map

fn prune_ratelimits(ratelimits: &DashMap<IpAddr, Ratelimit>) {
    ratelimits.retain(|_, counter| {
        let timestamp = counter.timestamp.load(Ordering::Acquire);
        timestamp + PRUNE_TIME * 1000 > utils::get_time()
    });
}
