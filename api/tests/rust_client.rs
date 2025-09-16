use std::time::Duration;

use eventsource_stream::Eventsource;
use rocket::futures::StreamExt;
use tinistream_client::{types::*, ClientClientExt, ClientStreamExt};

mod common;

use crate::common::{setup_backend_client, setup_frontend_client, setup_rocket};

#[tokio::test]
async fn server() -> Result<(), tokio::io::Error> {
    let (rocket, port, shutdown) = setup_rocket().await?;
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
    rocket.await.unwrap().expect("Rocket failed to shutdown");

    Ok(())
}

#[tokio::test]
async fn client() -> Result<(), std::io::Error> {
    let (rocket, port, shutdown) = setup_rocket().await?;
    let backend_client = setup_backend_client(port);

    // Create stream and get token
    let key = rand::random::<u16>().to_string();
    let res = backend_client
        .create_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await
        .unwrap();
    let token = res.into_inner().token;

    // Spawn task to add events to the Redis stream on an interval
    let task_key = key.clone();
    let add_events_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        for _ in 0..10 {
            interval.tick().await;
            let test_event = AddEvent::builder()
                .data("test_data".to_owned())
                .event("test_event");
            let body = AddEventsRequest::builder()
                .key(&task_key)
                .events(vec![test_event.try_into().unwrap()]);
            let _ = backend_client.add_events().body(body).send().await;
        }
        let _ = backend_client
            .end_stream()
            .body(StreamRequest { key: task_key })
            .send()
            .await;
    });

    // Create frontend client
    let client = setup_frontend_client(port, &token);

    // Delay a bit before connecting, to test that old events are still received
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Connect to SSE stream
    let res = client.connect_sse().key(&key).send().await.unwrap();
    assert!(res.status().is_success());
    assert!(res.headers().get("Content-Type").unwrap() == "text/event-stream");

    // Read events
    let mut events = Vec::new();
    let mut errors = Vec::new();
    let mut stream = res.into_inner_stream().eventsource();
    while let Some(res) = stream.next().await {
        match res {
            Ok(ev) => events.push(ev),
            Err(err) => errors.push(err),
        }
    }

    // Shutdown events task and rocket
    add_events_task.await.expect("should complete");
    shutdown.notify();
    rocket.await.unwrap().expect("Rocket failed to shutdown");

    // Make sure all events were received
    assert_eq!(events.len(), 12);
    assert_eq!(errors.len(), 0);
    assert_eq!(events[0].event, "start");
    assert_eq!(events[11].event, "end");
    for i in 1..=10 {
        assert!(events[i].id.len() > 0);
        assert_eq!(events[i].data, "test_data");
        assert_eq!(events[i].event, "test_event");
    }

    Ok(())
}
