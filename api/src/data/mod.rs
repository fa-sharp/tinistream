mod json_stream;

pub use json_stream::JsonStream;

const MAX_STREAM_SIZE: usize = 512 * 1024; // 512 KB
