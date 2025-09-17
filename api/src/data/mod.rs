mod json_stream;
mod ws_stream;

pub use json_stream::*;
pub use ws_stream::*;

const MAX_STREAM_SIZE: usize = 512 * 1024; // 512 KB
