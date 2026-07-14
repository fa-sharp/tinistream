use std::sync::LazyLock;

use fred::{clients::Client, prelude::FredResult, types::scripts::Script};

use crate::redis::{AddEvent, StreamStatus, constants, types::RedisStr};

/// Lua scripts for atomic Redis stream mutations. The scripts return
/// `nil` (i.e. `None`) when the stream state does not allow the mutation.
pub(super) struct RedisScripts;

impl RedisScripts {
    /// Start and activate a stream.
    ///
    /// Returns the Redis stream ID for the start event. Returns `None` if the
    /// stream is already active. If an inactive stream exists at the same key,
    /// the script deletes the old stream and metadata before creating the new
    /// stream.
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

        START_STREAM_SCRIPT
            .evalsha_with_reload(&client, (stream_key, meta_key), args)
            .await
    }

    /// Write a batch of events to an active stream.
    //
    /// Returns the Redis stream IDs for all written events. Returns `None` if
    /// the stream is not active, without writing any events.
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

        WRITE_EVENTS_SCRIPT
            .evalsha_with_reload(client, (stream_key, meta_key), args)
            .await
    }

    /// Write a terminal event and mark the stream inactive.
    ///
    /// Returns the Redis stream ID for the terminal event. Returns `None` if
    /// the stream is not active, without appending a terminal event.
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

        FINISH_STREAM_SCRIPT
            .evalsha_with_reload(&client, (stream_key, meta_key), args)
            .await
    }
}

/// Atomically create a stream unless it is already active.
///
/// Key contract:
/// - `KEYS[1]`: Redis stream key
/// - `KEYS[2]`: stream metadata hash key
///
/// Argument contract:
/// - `ARGV[1]`: metadata status field name
/// - `ARGV[2]`: active status value
/// - `ARGV[3]`: stream/meta TTL in seconds
/// - `ARGV[4]`: stream entry event field name
/// - `ARGV[5]`: start event value
///
/// Return contract:
/// - stream ID for the start event when created
/// - `nil` when the stream is already active
static START_STREAM_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    let lua = r#"
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
    Script::from_lua(lua)
});

/// Atomically write a batch of events if the stream is active.
///
/// Key contract:
/// - `KEYS[1]`: Redis stream key
/// - `KEYS[2]`: stream metadata hash key
///
/// Fixed argument contract:
/// - `ARGV[1]`: metadata status field name
/// - `ARGV[2]`: active status value
/// - `ARGV[3]`: approximate stream max length
/// - `ARGV[4]`: stream entry event field name
/// - `ARGV[5]`: stream entry data field name
///
/// Repeated event argument contract, starting at `ARGV[6]`:
/// - event name
/// - data flag: `"1"` means include the data field, `"0"` means omit it
/// - data value, or an empty placeholder when the flag is `"0"`
///
/// Return contract:
/// - array of stream IDs for the written events
/// - `nil` when the stream is not active
static WRITE_EVENTS_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    let lua = r#"
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
    Script::from_lua(lua)
});

/// Atomically append a terminal event and mark a stream inactive.
///
/// Key contract:
/// - `KEYS[1]`: Redis stream key
/// - `KEYS[2]`: stream metadata hash key
///
/// Argument contract:
/// - `ARGV[1]`: metadata status field name
/// - `ARGV[2]`: active status value
/// - `ARGV[3]`: final status value
/// - `ARGV[4]`: stream entry event field name
/// - `ARGV[5]`: terminal event value
///
/// Return contract:
/// - stream ID for the terminal event
/// - `nil` when the stream is not active
static FINISH_STREAM_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    let lua = r#"
if redis.call('HGET', KEYS[2], ARGV[1]) ~= ARGV[2] then
  return nil
end

local id = redis.call('XADD', KEYS[1], '*', ARGV[4], ARGV[5])
redis.call('HSET', KEYS[2], ARGV[1], ARGV[3])

return id
"#;
    Script::from_lua(lua)
});
