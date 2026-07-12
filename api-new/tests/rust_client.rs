use tinistream_client::{
    ClientEventsExt, ClientStreamExt,
    types::{AddEvent, AddEventsRequest, StreamRequest},
};

use crate::common::{setup_backend_client, setup_http_server};

mod common;

#[tokio::test]
async fn server() -> anyhow::Result<()> {
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

    // Shutdown server
    shutdown.await;

    Ok(())
}
