use std::sync::LazyLock;

use fred::{prelude::FredResult, types::scripts::Script};

use crate::redis::{AddEvent, ExclusiveClient, StreamService, constants, types::RedisStr};

/// Lua scripts for atomic writes
static SCRIPTS: LazyLock<RedisScripts> = LazyLock::new(RedisScripts::new);

/// A stream writer with an exclusive lock on a Redis connection, for
/// long-running write operations (e.g. for ingesting events into Redis)
pub struct RedisWriter {
    client: ExclusiveClient,
    max_len: u32,
    stream: StreamService,
}

impl RedisWriter {
    pub fn new(client: ExclusiveClient, max_len: u32, stream: StreamService) -> Self {
        Self {
            client,
            max_len,
            stream,
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

        let mut max_len_buffer = itoa::Buffer::new();
        let mut args = vec![
            constants::META_STATUS_FIELD,
            constants::StreamStatus::Active.as_str(),
            max_len_buffer.format(self.max_len),
            constants::EVENT_KEY,
            constants::DATA_KEY,
        ];
        args.extend(events.iter().flat_map(|ev| match ev.data.as_ref() {
            Some(data) => [&ev.event, "1", data],
            None => [&ev.event, "0", ""],
        }));

        SCRIPTS
            .write_events
            .evalsha_with_reload(&self.client, (stream_key, meta_key), args)
            .await
    }
}

/// Redis Lua scripts for atomic writes
struct RedisScripts {
    write_events: Script,
}

impl RedisScripts {
    pub fn new() -> Self {
        Self {
            write_events: Script::from_lua(WRITE_EVENTS_SCRIPT),
        }
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
