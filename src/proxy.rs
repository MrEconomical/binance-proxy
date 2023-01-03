// Imports

use crate::config::ENDPOINT;

use reqwest::Client;
use warp::path::FullPath;
use warp::Reply;

// Route GET request

pub async fn get(
    client: Client,
    path: FullPath,
    query: Option<String>,
) -> Result<Box<dyn Reply>, ()> {
    // Send GET request

    let response = client
        .get(if let Some(q) = query {
            String::from(ENDPOINT) + path.as_str() + "?" + &q
        } else {
            String::from(ENDPOINT) + path.as_str()
        })
        .send()
        .await
        .map_err(|_| ())?;
    let status = response.status();
    let content_type = response.headers().get("content-type").map(|h| h.clone());

    // Get response data

    let reply = warp::reply::with_status(response.text().await.map_err(|_| ())?, status);
    Ok(if let Some(typ) = content_type {
        Box::new(warp::reply::with_header(reply, "content-type", typ))
    } else {
        Box::new(reply)
    })
}
