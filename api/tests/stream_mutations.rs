use tinistream_client::{
    ClientIngestExt, ClientStreamExt, Error, ResponseValue,
    types::{AddEvent, AddEventsRequest, ErrorResponse, StreamRequest},
};

use crate::common::{setup_backend_client, setup_http_server};

mod common;

fn assert_error_status<T: std::fmt::Debug>(
    result: Result<ResponseValue<T>, Error<ErrorResponse>>,
    expected: reqwest::StatusCode,
) {
    let err = result.expect_err("expected request to fail");
    assert_eq!(err.status(), Some(expected));
}

#[tokio::test]
async fn mutations_are_state_guarded() -> anyhow::Result<()> {
    let (port, _server, shutdown) = setup_http_server().await?;
    let client = setup_backend_client(port);
    let key = rand::random::<u16>().to_string();

    let res = client
        .create_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await
        .expect("should create stream");
    assert!(res.status().is_success());

    let duplicate_create = client
        .create_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await;
    assert_error_status(duplicate_create, reqwest::StatusCode::BAD_REQUEST);

    let test_event = AddEvent::builder()
        .data("test_data".to_owned())
        .event("test_event");
    let body = AddEventsRequest::builder()
        .key(&key)
        .events(vec![test_event.try_into().unwrap()]);
    let res = client
        .add_events()
        .body(body)
        .send()
        .await
        .expect("should add event");
    assert_eq!(res.num_events, 1);

    let res = client
        .end_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await
        .expect("should end stream");
    assert!(res.status().is_success());

    let duplicate_end = client
        .end_stream()
        .body(StreamRequest::builder().key(&key))
        .send()
        .await;
    assert_error_status(duplicate_end, reqwest::StatusCode::NOT_FOUND);

    let test_event = AddEvent::builder()
        .data("late_data".to_owned())
        .event("late_event");
    let body = AddEventsRequest::builder()
        .key(&key)
        .events(vec![test_event.try_into().unwrap()]);
    let add_after_end = client.add_events().body(body).send().await;
    assert_error_status(add_after_end, reqwest::StatusCode::BAD_REQUEST);

    shutdown.await.expect("failed to shutdown server");

    Ok(())
}
