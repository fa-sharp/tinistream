use tinistreamer::build_rocket;
use tinistreamer_client::{types::*, Client, ClientStreamExt};

#[tokio::test]
async fn stream() {
    let rocket = build_rocket().ignite().await.expect("Failed to ignite");
    let shutdown = rocket.shutdown();
    let rocket_handle = tokio::spawn(rocket.launch());

    let api_key = dotenvy::var("STREAMER_API_KEY").expect("API key not set");
    let mut api_key_header = reqwest::header::HeaderMap::new();
    api_key_header.insert("X-API-KEY", api_key.parse().unwrap());

    let http_client = reqwest::Client::builder()
        .default_headers(api_key_header)
        .build()
        .unwrap();
    let client = Client::new_with_client("http://localhost:8000", http_client);
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
        res.url,
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
    let events = std::iter::repeat(test_event).take(5).collect::<Vec<_>>();
    let res = client
        .add_events()
        .body(AddEventsRequest::builder().key(&key).events(events))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());
    assert_eq!(res.ids.len(), 5);

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

    // Shutdown rocket
    shutdown.notify();
    rocket_handle.await.expect("Failed to shutdown").unwrap();
}
