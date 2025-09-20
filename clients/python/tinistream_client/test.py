from tinistream_client.api.stream import create_stream
from tinistream_client.client import AuthenticatedClient
from tinistream_client.models import ErrorMessage, StreamRequest

client = AuthenticatedClient(base_url="https://my.tinistreamer.com", api_key="my_api_key")

res = create_stream.sync(client=client, body=StreamRequest(key="my_key"))
if isinstance(res, ErrorMessage):
    print(f"Error: {res.code} {res.message}")
elif res:
    print(f"Stream created: URL: {res.sse_url} Token: {res.token}")
