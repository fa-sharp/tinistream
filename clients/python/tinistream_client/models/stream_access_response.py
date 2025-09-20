from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="StreamAccessResponse")


@_attrs_define
class StreamAccessResponse:
    """
    Attributes:
        sse_url (str): URL for the client to connect to the stream via SSE
        ws_url (str): URL for the client to connect to the stream via WebSocket
        token (str): Client token to access the stream. Can be used as a Bearer token in the Authorization header
            (recommended) or as the `token` query parameter.
    """

    sse_url: str
    ws_url: str
    token: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        sse_url = self.sse_url

        ws_url = self.ws_url

        token = self.token

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "sse_url": sse_url,
                "ws_url": ws_url,
                "token": token,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        sse_url = d.pop("sse_url")

        ws_url = d.pop("ws_url")

        token = d.pop("token")

        stream_access_response = cls(
            sse_url=sse_url,
            ws_url=ws_url,
            token=token,
        )

        stream_access_response.additional_properties = d
        return stream_access_response

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
