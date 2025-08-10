//! Server-Sent Events (SSE) parsing for streaming responses

use bytes::{Bytes, BytesMut};
use futures::stream::Stream;
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Represents a single SSE event
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

pin_project! {
    /// A stream that parses SSE events from a byte stream
    pub struct SseStream<S> {
        #[pin]
        inner: S,
        buffer: BytesMut,
    }
}

impl<S> SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: stream,
            buffer: BytesMut::new(),
        }
    }

    fn parse_event(data: &str) -> Option<SseEvent> {
        let mut event = None;
        let mut event_data = Vec::new();
        let mut id = None;
        let mut retry = None;

        for line in data.lines() {
            if line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim_start();
                match key {
                    "event" => event = Some(value.to_string()),
                    "data" => event_data.push(value.to_string()),
                    "id" => id = Some(value.to_string()),
                    "retry" => retry = value.parse().ok(),
                    _ => {}
                }
            } else if line.starts_with(':') {
                // Comment, ignore
                continue;
            }
        }

        if !event_data.is_empty() {
            Some(SseEvent {
                event,
                data: event_data.join("\n"),
                id,
                retry,
            })
        } else {
            None
        }
    }
}

impl<S> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    type Item = Result<SseEvent, reqwest::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            // Try to parse an event from the buffer
            if let Some(pos) = this.buffer.windows(2).position(|w| w == b"\n\n") {
                let event_data = this.buffer.split_to(pos + 2);
                let event_str = String::from_utf8_lossy(&event_data);
                
                if let Some(event) = Self::parse_event(&event_str) {
                    return Poll::Ready(Some(Ok(event)));
                }
            }

            // Read more data from the stream
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    this.buffer.extend_from_slice(&bytes);
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => {
                    // Stream ended, try to parse any remaining data
                    if !this.buffer.is_empty() {
                        let remaining = this.buffer.split();
                        let event_str = String::from_utf8_lossy(&remaining);
                        
                        if let Some(event) = Self::parse_event(&event_str) {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Helper function to create an SSE stream from a response
pub fn sse_stream(response: reqwest::Response) -> impl Stream<Item = Result<SseEvent, reqwest::Error>> {
    SseStream::new(response.bytes_stream())
}