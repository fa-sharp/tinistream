use fred::prelude::{FredResult, LuaInterface};

use crate::redis::{AddEvent, ExclusiveClient, StreamService, constants, types::RedisStr};

/// A stream writer with an exclusive lock on a Redis connection, for
/// long-running write operations (e.g. for ingesting events into Redis)
pub struct RedisWriter {
    client: ExclusiveClient,
    stream: StreamService,
    max_len: String,
}

impl RedisWriter {
    pub fn new(client: ExclusiveClient, max_len: u32, stream_service: StreamService) -> Self {
        Self {
            client,
            max_len: max_len.to_string(),
            stream: stream_service,
        }
    }

    /// Write events to the stream, with an atomic check if the stream is active.
    /// Returns the IDs of the written events, or `None` if the stream is not active.
    pub async fn write_events(
        &self,
        key: &str,
        events: Vec<AddEvent>,
    ) -> FredResult<Option<Vec<RedisStr>>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        let mut args = vec![
            constants::META_STATUS_FIELD,
            constants::StreamStatus::Active.as_str(),
            &self.max_len,
            constants::EVENT_KEY,
            constants::DATA_KEY,
        ];
        args.extend(events.iter().flat_map(|ev| match ev.data.as_ref() {
            Some(data) => [&ev.event, "1", data],
            None => [&ev.event, "0", ""],
        }));

        self.client
            .eval(WRITE_EVENTS_SCRIPT, (stream_key, meta_key), args)
            .await
    }
}

/// Lua script to atomically check for an active stream and write events.
const WRITE_EVENTS_SCRIPT: &str = r#"
if redis.call('HGET', KEYS[2], ARGV[1]) ~= ARGV[2] then
  return nil
end

local ids = {}
local arg_index = 6
while arg_index <= #ARGV do
  local event = ARGV[arg_index]
  local has_data = ARGV[arg_index + 1]
  local data = ARGV[arg_index + 2]
  arg_index = arg_index + 3

  local command = {'XADD', KEYS[1], 'MAXLEN', '~', ARGV[3], '*', ARGV[4], event}
  if has_data == '1' then
    table.insert(command, ARGV[5])
    table.insert(command, data)
  end

  table.insert(ids, redis.call(unpack(command)))
end

return ids
"#;
