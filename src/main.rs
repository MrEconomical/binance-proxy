// Imports

mod config;
mod proxy;
mod server;
mod utils;

#[allow(unused_imports)]
use config::{CERT, DOMAIN, HOST};

#[allow(unused_imports)]
use std::convert::Infallible;

#[allow(unused_imports)]
use warp::http::Uri;
#[allow(unused_imports)]
use warp::path::FullPath;
#[allow(unused_imports)]
use warp::Filter;

// Start proxy server

#[cfg(debug_assertions)]
#[tokio::main]
async fn main() {
    server::create_server().run(([127, 0, 0, 1], 3000)).await;
}

#[cfg(not(debug_assertions))]
#[tokio::main]
async fn main() {
    // Start HTTPS server

    let https_server = server::create_server()
        .tls()
        .cert_path(CERT.0)
        .key_path(CERT.1)
        .run((HOST, 443));

    // Redirect HTTP requests to HTTPS

    let redirect = warp::any()
        .and(warp::path::full())
        .and(
            warp::query::raw()
                .map(Some)
                .or_else(|_| async { Ok::<_, Infallible>((None,)) }),
        )
        .map(|path: FullPath, query: Option<String>| {
            warp::redirect::permanent(
                Uri::builder()
                    .scheme("https")
                    .authority(DOMAIN)
                    .path_and_query(if let Some(q) = query {
                        path.as_str().to_owned() + "?" + &q
                    } else {
                        path.as_str().to_owned()
                    })
                    .build()
                    .unwrap(),
            )
        });
    let http_server = warp::serve(redirect).run((HOST, 80));

    tokio::join!(https_server, http_server);
}
