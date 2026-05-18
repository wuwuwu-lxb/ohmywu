pub mod anthropic;
pub mod gemini;
pub mod openai_chat;
pub mod ollama;

use async_stream::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;

use crate::error::LlmError;
use crate::types::ChatStreamChunk;

pub fn buffered_line_stream<F>(
    response: reqwest::Response,
    parser: F,
) -> Pin<Box<dyn Stream<Item = std::result::Result<ChatStreamChunk, LlmError>> + Send>>
where
    F: Fn(&str) -> std::result::Result<Option<ChatStreamChunk>, LlmError> + Send + Sync + 'static,
{
    let stream = stream! {
        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(item) = byte_stream.next().await {
            match item {
                Err(e) => {
                    yield Err(LlmError::Connection(e.to_string()));
                    break;
                }
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].trim_end_matches('\r').to_string();
                        buffer.drain(..=pos);
                        match parser(&line) {
                            Ok(Some(chunk)) => yield Ok(chunk),
                            Ok(None) => {}
                            Err(e) => yield Err(e),
                        }
                    }
                }
            }
        }

        let trailing = buffer.trim();
        if !trailing.is_empty() {
            match parser(trailing) {
                Ok(Some(chunk)) => yield Ok(chunk),
                Ok(None) => {}
                Err(e) => yield Err(e),
            }
        }
    };

    Box::pin(stream)
}
