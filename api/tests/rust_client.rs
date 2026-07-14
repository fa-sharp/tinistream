use std::{collections::HashMap, time::Duration};

use eventsource_stream::Eventsource;
use futures::{SinkExt, StreamExt};
use reqwest_websocket::Upgrade;
use serde::Deserialize;
use tinistream_client::{
    ClientIngestExt, ClientStreamExt,
    types::{AddEvent, AddEventsRequest, StreamRequest},
};

use crate::common::{setup_backend_client, setup_http_server};

mod common;

/// Setup reqwest client for frontend API requests
pub fn setup_frontend_client(token: &str) -> reqwest::Client {
    let mut token_header = reqwest::header::HeaderMap::new();
    token_header.insert("Authorization", format!("Bearer {token}").parse().unwrap());
    reqwest::Client::builder()
        .default_headers(token_header)
        .build()
        .expect("build client")
}

/// Spawn task to add 10 `(test_event, test_data)` events to the Redis stream on an interval
pub fn add_events_task(
    client: tinistream_client::Client,
    key: &str,
) -> tokio::task::JoinHandle<()> {
    use tinistream_client::types::{AddEvent, AddEventsRequest, StreamRequest};
    use tinistream_client::{ClientIngestExt, ClientStreamExt};

    let key = key.to_owned();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        for _ in 0..10 {
            interval.tick().await;
            let test_event = AddEvent::builder()
                .data("test_data".to_owned())
                .event("test_event");
            let body = AddEventsRequest::builder()
                .key(&key)
                .events(vec![test_event.try_into().unwrap()]);
            if let Err(e) = client.add_events().body(body).send().await {
                eprintln!("Error in add events task: {e}");
            }
        }
        let _ = client
            .end_stream()
            .body(StreamRequest::builder().key(key))
            .send()
            .await;
    })
}

#[tokio::test]
async fn basic() -> anyhow::Result<()> {
    let (port, _server, shutdown) = setup_http_server().await?;
    let client = setup_backend_client(port);
    let key = rand::random::<u16>().to_string();

    // Create stream
    let res = client
        .create_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());
    assert_eq!(
        res.sse_url,
        format!("http://localhost:8000/api/client/sse?key={}", key)
    );

    // List streams
    let res = client.list_streams().pattern(&key).send().await.unwrap();
    assert!(res.status().is_success());
    assert_eq!(res[0].key, key);
    assert_eq!(res[0].length, 1);

    // Add events
    let test_event: AddEvent = AddEvent::builder()
        .data("test_data".to_owned())
        .event("test_event")
        .try_into()
        .unwrap();
    let events = std::iter::repeat_n(test_event, 5).collect::<Vec<_>>();
    let res = client
        .add_events()
        .body(AddEventsRequest::builder().key(&key).events(events))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());
    assert_eq!(res.num_events, 5);

    // End stream
    let res = client
        .end_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Check that stream has ended
    let res = client.list_streams().pattern(&key).send().await.unwrap();
    assert!(res.status().is_success());
    assert_eq!(res.len(), 0);

    // Shutdown server
    shutdown.await.expect("failed to shutdown server");

    Ok(())
}

#[tokio::test]
async fn client_sse() -> anyhow::Result<()> {
    let (port, _server, shutdown) = setup_http_server().await?;
    let client = setup_backend_client(port);

    // Create stream and get token
    let key = rand::random::<u16>().to_string();
    let res = client
        .create_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await
        .expect("Should create stream")
        .into_inner();

    // Spawn task to add events to the Redis stream on an interval
    let add_events_task = add_events_task(client, &key);

    // Create frontend client
    let frontend_client = setup_frontend_client(&res.token);

    // Delay a bit before connecting, to test that old events are still received
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Connect to SSE stream
    let res = frontend_client
        .get(format!("http://localhost:{port}/api/client/sse?key={key}"))
        .send()
        .await
        .expect("should connect to SSE stream");
    assert!(res.status().is_success());
    assert!(res.headers().get("Content-Type").unwrap() == "text/event-stream");

    // Read events
    let mut events = Vec::new();
    let mut errors = Vec::new();
    let mut stream = res.bytes_stream().eventsource();
    while let Some(res) = stream.next().await {
        match res {
            Ok(ev) => events.push(ev),
            Err(err) => errors.push(err),
        }
    }

    // Shutdown events task and server
    add_events_task.await.expect("should complete");
    shutdown.await.expect("failed to shutdown server");

    // Make sure all events were received
    assert_eq!(errors.len(), 0, "Errors during stream: {errors:?}");
    assert_eq!(events.len(), 12);
    assert_eq!(events[0].event, "start");
    assert_eq!(events[11].event, "end");
    for event in events.iter().take(11).skip(1) {
        assert!(!event.id.is_empty());
        assert_eq!(event.data, "test_data");
        assert_eq!(event.event, "test_event");
    }

    Ok(())
}

#[tokio::test]
async fn client_websocket() -> anyhow::Result<()> {
    let (port, _server, shutdown) = setup_http_server().await?;
    let client = setup_backend_client(port);

    // Create stream and get token
    let key = rand::random::<u16>().to_string();
    let res = client
        .create_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await
        .expect("should create stream");
    let token = res.into_inner().token;

    // Spawn task to add events to the Redis stream on an interval
    let add_events_task = add_events_task(client, &key);

    // Create frontend client
    let client = setup_frontend_client(&token);

    // Delay a bit before connecting, to test that old events are still received
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Connect to WebSocket stream
    let res = client
        .get(format!("http://localhost:{port}/api/client/ws?key={key}"))
        .upgrade()
        .send()
        .await
        .expect("should connect to WebSocket stream");
    assert!(res.status() == reqwest::StatusCode::SWITCHING_PROTOCOLS);

    // Read events
    let mut events = Vec::new();
    let mut errors = Vec::new();
    let mut stream = res.into_websocket().await.unwrap().enumerate();
    while let Some((index, res)) = stream.next().await {
        match res {
            Ok(msg) => match msg {
                reqwest_websocket::Message::Text(t) => {
                    if index == 0 {
                        let prev_events = serde_json::from_str::<PrevEvents>(&t)
                            .expect("should parse previous events");
                        assert_eq!(prev_events.event, "prev_events");
                        events.extend(prev_events.data);
                    } else {
                        let parsed = serde_json::from_str::<HashMap<String, String>>(&t)
                            .expect("should parse event");
                        events.push(parsed);
                    }
                }
                reqwest_websocket::Message::Close { .. } => {
                    break;
                }
                msg => panic!("Unexpected message type: {msg:?}"),
            },
            Err(err) => errors.push(err),
        }
    }
    stream.close().await.expect("should close connection");

    // Shutdown events task and server
    add_events_task.await.expect("should complete");
    shutdown.await.expect("failed to shutdown server");

    // Make sure all events were received
    assert_eq!(errors.len(), 0, "Errors during stream: {errors:?}");
    assert_eq!(events.len(), 12);
    assert_eq!(events[0]["event"], "start");
    assert_eq!(events[11]["event"], "end");
    for event in events.iter().take(11).skip(1) {
        assert!(!event["id"].is_empty());
        assert_eq!(event["data"], "test_data");
        assert_eq!(event["event"], "test_event");
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PrevEvents {
    event: String,
    data: Vec<HashMap<String, String>>,
}
