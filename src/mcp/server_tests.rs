use super::*;

async fn round_trip(line: &str) -> Option<Value> {
    let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
    let (tx, mut rx) = mpsc::channel(8);
    dispatch(line, &permits, &tx).await.unwrap();
    rx.recv()
        .await
        .map(|text| serde_json::from_str(&text).unwrap())
}

#[tokio::test]
async fn oversized_lines_are_rejected_without_being_parsed() {
    let response = round_trip(&"x".repeat(MAX_LINE_BYTES + 1))
        .await
        .expect("oversized frames must still get a reply");
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn batch_arrays_get_an_explicit_error() {
    let response = round_trip(&json!([1, 2]).to_string())
        .await
        .expect("batches must be answered, not dropped");
    assert_eq!(response["error"]["code"], -32600);
}

#[tokio::test]
async fn non_object_frames_get_an_invalid_request_error() {
    for frame in ["42", "\"str\""] {
        let response = round_trip(frame)
            .await
            .expect("non-object frames must be answered");
        assert_eq!(response["error"]["code"], -32600, "{frame}");
        assert_eq!(response["id"], Value::Null, "{frame}");
    }
}

#[tokio::test]
async fn malformed_json_yields_parse_error() {
    let response = round_trip("this is not json")
        .await
        .expect("parse errors must be answered");
    assert_eq!(response["error"]["code"], -32700);
}

#[tokio::test]
async fn ping_round_trips_through_the_writer_queue() {
    let request = json!({"jsonrpc": "2.0", "id": 11, "method": "ping"}).to_string();
    let response = round_trip(&request).await.expect("ping must be answered");
    assert_eq!(response["id"], 11);
    assert_eq!(response["result"], json!({}));
}

#[tokio::test]
async fn notifications_produce_no_response() {
    let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
    let (tx, mut rx) = mpsc::channel(8);
    let notification = json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string();
    dispatch(&notification, &permits, &tx).await.unwrap();
    drop(tx);
    assert!(rx.recv().await.is_none(), "notifications expect no reply");
}
