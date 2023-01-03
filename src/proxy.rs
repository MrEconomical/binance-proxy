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
            ENDPOINT.to_owned() + path.as_str() + "?" + &q
        } else {
            ENDPOINT.to_owned() + path.as_str()
        })
        .send()
        .await
        .map_err(|_| ())?;
    let status = response.status();
    let content_type = response.headers().get("content-type").cloned();

    // Get response data

    let reply = warp::reply::with_status(response.text().await.map_err(|_| ())?, status);
    Ok(if let Some(typ) = content_type {
        Box::new(warp::reply::with_header(reply, "content-type", typ))
    } else {
        Box::new(reply)
    })
}
