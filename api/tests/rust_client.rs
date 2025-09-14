use tinistreamer_client::{types::*, Client, ClientStreamExt};

#[tokio::test]
async fn stream() {
    let mut api_key_header = reqwest::header::HeaderMap::new();
    api_key_header.insert("X-API-KEY", "api-key-123".parse().unwrap());

    let http_client = reqwest::Client::builder()
        .default_headers(api_key_header)
        .build()
        .unwrap();
    let client = Client::new_with_client("http://localhost:8080/api", http_client);
    let key = "test_key";

    // Create stream
    let res = client
        .create_stream()
        .body(StreamRequest::builder().key(key))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());
    assert_eq!(res.url, "http://localhost:8080/api/client/sse?key=test_key");

    // List streams
    let res = client.list_streams().pattern(key).send().await.unwrap();
    assert!(res.status().is_success());
    assert_eq!(res[0].key, key);
    assert_eq!(res[0].length, 1);

    // Add events
    let test_event: AddEvent = AddEvent::builder()
        .data("test_data")
        .event("test_event")
        .try_into()
        .unwrap();
    let events = std::iter::repeat(test_event).take(5).collect::<Vec<_>>();
    let res = client
        .add_events()
        .body(AddEventsRequest::builder().key(key).events(events))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());
    assert_eq!(res.ids.len(), 5);

    // End stream
    let res = client
        .end_stream()
        .body(StreamRequest::builder().key(key))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success());

    // Check that stream has ended
    let res = client.list_streams().pattern(key).send().await.unwrap();
    assert!(res.status().is_success());
    assert_eq!(res.len(), 0);
}
