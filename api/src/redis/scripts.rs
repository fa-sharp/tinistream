use std::sync::LazyLock;

use fred::{clients::Client, prelude::FredResult, types::scripts::Script};

use crate::redis::{AddEvent, StreamStatus, constants, types::RedisStr};

/// Lua scripts for atomic Redis stream mutations.
pub(super) static SCRIPTS: LazyLock<RedisScripts> = LazyLock::new(RedisScripts::new);

pub(super) struct RedisScripts {
    start_stream: Script,
    write_events: Script,
    finish_stream: Script,
}

impl RedisScripts {
    fn new() -> Self {
        Self {
            start_stream: Script::from_lua(START_STREAM_SCRIPT),
            write_events: Script::from_lua(WRITE_EVENTS_SCRIPT),
            finish_stream: Script::from_lua(FINISH_STREAM_SCRIPT),
        }
    }

    /// Start/activate the stream and returns the ID of the start event. Returns
    /// `None` if stream is already active.
    pub(super) async fn start_stream(
        &self,
        client: &Client,
        stream_key: &str,
        meta_key: &str,
        ttl: u32,
    ) -> FredResult<Option<RedisStr>> {
        let mut ttl_buffer = itoa::Buffer::new();
        let args = [
            constants::META_STATUS_FIELD,
            constants::StreamStatus::Active.as_str(),
            ttl_buffer.format(ttl),
            constants::EVENT_KEY,
            constants::START,
        ];

        self.start_stream
            .evalsha_with_reload(&client, (stream_key, meta_key), args)
            .await
    }

    /// Write events to the stream. Returns `None` if stream is not active.
    pub(super) async fn write_events(
        &self,
        client: &Client,
        stream_key: &str,
        meta_key: &str,
        max_len: u32,
        events: Vec<AddEvent>,
    ) -> FredResult<Option<Vec<RedisStr>>> {
        let mut max_len_buffer = itoa::Buffer::new();
        let mut args = vec![
            constants::META_STATUS_FIELD,
            constants::StreamStatus::Active.as_str(),
            max_len_buffer.format(max_len),
            constants::EVENT_KEY,
            constants::DATA_KEY,
        ];
        args.extend(events.iter().flat_map(|ev| match ev.data.as_ref() {
            Some(data) => [&ev.event, "1", data],
            None => [&ev.event, "0", ""],
        }));

        self.write_events
            .evalsha_with_reload(client, (stream_key, meta_key), args)
            .await
    }

    /// Write end event and mark stream as inactive
    pub(super) async fn finish_stream(
        &self,
        client: &Client,
        stream_key: &str,
        meta_key: &str,
        status: StreamStatus,
        event: &str,
    ) -> FredResult<Option<RedisStr>> {
        let args = [
            constants::META_STATUS_FIELD,
            constants::StreamStatus::Active.as_str(),
            status.as_str(),
            constants::EVENT_KEY,
            event,
        ];

        self.finish_stream
            .evalsha_with_reload(&client, (stream_key, meta_key), args)
            .await
    }
}

/// Lua script to atomically create a stream unless it is already active.
const START_STREAM_SCRIPT: &str = r#"
if redis.call('HGET', KEYS[2], ARGV[1]) == ARGV[2] then
  return nil
end

redis.call('DEL', KEYS[1], KEYS[2])
local id = redis.call('XADD', KEYS[1], '*', ARGV[4], ARGV[5])
redis.call('EXPIRE', KEYS[1], ARGV[3])
redis.call('HSET', KEYS[2], ARGV[1], ARGV[2])
redis.call('EXPIRE', KEYS[2], ARGV[3])

return id
"#;

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

/// Lua script to atomically append a terminal event and mark a stream inactive.
const FINISH_STREAM_SCRIPT: &str = r#"
if redis.call('HGET', KEYS[2], ARGV[1]) ~= ARGV[2] then
  return nil
end

local id = redis.call('XADD', KEYS[1], '*', ARGV[4], ARGV[5])
redis.call('HSET', KEYS[2], ARGV[1], ARGV[3])

return id
"#;
