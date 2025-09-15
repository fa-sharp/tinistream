use std::pin::Pin;

use rocket::{
    async_trait,
    data::{FromData, Outcome},
    Request,
};
use rocket_okapi::request::OpenApiFromData;
use tokio_stream::{Stream, StreamExt};
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};

use crate::api::stream::AddEvent;

const MAX_STREAM_SIZE: usize = 1 * 1024 * 1024; // 1 MB

/// Data guard for JSON streams
pub struct JsonStream<'r> {
    pub stream: Pin<Box<dyn Stream<Item = Result<AddEvent, LinesCodecError>> + Send + 'r>>,
}

#[async_trait]
impl<'r> FromData<'r> for JsonStream<'r> {
    type Error = &'static str;

    async fn from_data(
        _req: &'r Request<'_>,
        data: rocket::Data<'r>,
    ) -> Outcome<'r, Self, Self::Error> {
        let data = data.open(MAX_STREAM_SIZE.into());
        let reader = tokio::io::BufReader::new(data);
        let line_reader = FramedRead::new(reader, LinesCodec::new());

        let stream = line_reader.filter_map(|line_result| match line_result {
            Ok(line) => {
                if line.trim_start().as_bytes().starts_with(b"{") {
                    if let Ok(ev) = serde_json::from_str::<AddEvent>(&line) {
                        return Some(Ok(ev));
                    }
                }
                None
            }
            Err(e) => Some(Err(e)),
        });

        Outcome::Success(JsonStream {
            stream: Box::pin(stream),
        })
    }
}

impl<'r> OpenApiFromData<'r> for JsonStream<'r> {
    fn request_body(
        _gen: &mut rocket_okapi::r#gen::OpenApiGenerator,
    ) -> rocket_okapi::Result<rocket_okapi::okapi::openapi3::RequestBody> {
        use rocket_okapi::okapi::openapi3;
        use schemars::schema;

        Ok(openapi3::RequestBody {
            description: Some(
                "JSON stream (stream of JSON strings separated by newlines)".to_string(),
            ),
            content: {
                let mut content = schemars::Map::new();
                content.insert(
                    "application/octet-stream".into(),
                    openapi3::MediaType {
                        schema: Some(openapi3::SchemaObject {
                            instance_type: Some(schema::SingleOrVec::Single(Box::new(
                                schema::InstanceType::String,
                            ))),
                            format: Some("binary".to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                );
                content
            },
            required: true,
            ..Default::default()
        })
    }
}
