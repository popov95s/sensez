use super::*;

#[tokio::test]
async fn oversized_frame_is_discarded_and_the_next_request_is_read() {
    let mut bytes = vec![b'x'; MAX_LINE_BYTES + 2];
    bytes.extend_from_slice(b"\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
    let mut reader = BufReader::with_capacity(4096, std::io::Cursor::new(bytes));
    let mut state = FrameState::default();
    assert!(matches!(
        read_frame(&mut reader, &mut state).await.unwrap(),
        Some(Frame::Oversized)
    ));
    assert!(matches!(
        read_frame(&mut reader, &mut state).await.unwrap(),
        Some(Frame::Line(line)) if line.contains("ping")
    ));
    assert!(read_frame(&mut reader, &mut state).await.unwrap().is_none());
}

#[tokio::test]
async fn partial_frame_survives_read_cancellation() {
    let (mut writer, reader) = tokio::io::duplex(32);
    let mut reader = BufReader::new(reader);
    let mut state = FrameState::default();
    writer.write_all(b"pin").await.unwrap();
    let timed_out = tokio::time::timeout(
        Duration::from_millis(10),
        read_frame(&mut reader, &mut state),
    )
    .await;
    assert!(timed_out.is_err());
    writer.write_all(b"g\n").await.unwrap();
    assert!(matches!(
        read_frame(&mut reader, &mut state).await.unwrap(),
        Some(Frame::Line(line)) if line == "ping"
    ));
}
