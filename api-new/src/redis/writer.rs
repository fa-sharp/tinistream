use fred::prelude::{FredResult, LuaInterface};

use crate::redis::{ExclusiveClientManager, StreamService, constants, types::RedisStr};

/// A stream writer with an exclusive lock on a Redis connection, for
/// long-running write operations (e.g. for ingesting events into Redis)
pub struct RedisWriter {
    client: deadpool::managed::Object<ExclusiveClientManager>,
    stream: StreamService,
    max_len: u32,
}

impl RedisWriter {
    pub fn new(
        client: deadpool::managed::Object<ExclusiveClientManager>,
        max_len: u32,
        stream_service: StreamService,
    ) -> Self {
        Self {
            client,
            max_len,
            stream: stream_service,
        }
    }

    /// Write events to the stream, while checking if the stream is active.
    /// Returns the IDs of the written events, or `None` if the stream is not active.
    pub async fn write_events(
        &self,
        key: &str,
        events: impl IntoIterator<Item = Vec<(&str, String)>>,
    ) -> FredResult<Option<Vec<RedisStr>>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);
        let mut args = vec![
            constants::META_STATUS_FIELD.to_owned(),
            constants::StreamStatus::Active.as_str().to_owned(),
            self.max_len.to_string(),
        ];

        for event in events {
            args.push(event.len().to_string());
            for (field, value) in event {
                args.push(field.to_owned());
                args.push(value);
            }
        }

        self.client
            .eval(WRITE_EVENTS_SCRIPT, (stream_key, meta_key), args)
            .await
    }
}

/// Lua script to atomatically check for an active stream and write events
const WRITE_EVENTS_SCRIPT: &str = r#"
if redis.call('HGET', KEYS[2], ARGV[1]) ~= ARGV[2] then
  return nil
end

local ids = {}
local arg_index = 4
while arg_index <= #ARGV do
  local field_count = tonumber(ARGV[arg_index])
  arg_index = arg_index + 1

  local fields = {}
  for _ = 1, field_count do
    table.insert(fields, ARGV[arg_index])
    table.insert(fields, ARGV[arg_index + 1])
    arg_index = arg_index + 2
  end

  local command = {'XADD', KEYS[1], 'MAXLEN', '~', ARGV[3], '*'}
  for _, field in ipairs(fields) do
    table.insert(command, field)
  end

  table.insert(ids, redis.call(unpack(command)))
end

return ids
"#;
