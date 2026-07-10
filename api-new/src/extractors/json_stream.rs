use axum::extract::FromRequest;
use futures::{TryStreamExt, stream::BoxStream};
use tokio_stream::StreamExt;
use tokio_util::{
    codec::{FramedRead, LinesCodec},
    io::StreamReader,
};

use crate::redis::AddEvent;

/// Extractor to read an incoming JSON Lines stream of events
pub struct JsonStream(pub BoxStream<'static, Result<AddEvent, std::io::Error>>);

impl<S: Send + Sync> FromRequest<S> for JsonStream {
    type Rejection = ();

    async fn from_request(
        req: axum::extract::Request,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let body_stream = req
            .into_body()
            .into_data_stream()
            .map_err(std::io::Error::other);
        let reader = tokio::io::BufReader::new(StreamReader::new(body_stream));
        let line_reader = FramedRead::new(reader, LinesCodec::new_with_max_length(5 * 1024));

        let stream = line_reader.filter_map(|line_result| match line_result {
            Ok(line) => {
                if line.trim().is_empty() {
                    None
                } else if line.trim_start().as_bytes().starts_with(b"{") {
                    match serde_json::from_str::<AddEvent>(&line) {
                        Ok(event) => Some(Ok(event)),
                        Err(err) => Some(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            err,
                        ))),
                    }
                } else {
                    Some(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "expected JSON object line",
                    )))
                }
            }
            Err(e) => match e {
                tokio_util::codec::LinesCodecError::Io(error) => Some(Err(error)),
                tokio_util::codec::LinesCodecError::MaxLineLengthExceeded => {
                    Some(Err(std::io::Error::other("max line length exceeded")))
                }
            },
        });

        Ok(Self(Box::pin(stream)))
    }
}
