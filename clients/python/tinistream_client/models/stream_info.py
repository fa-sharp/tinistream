from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="StreamInfo")


@_attrs_define
class StreamInfo:
    """Information about the stream

    Attributes:
        key (str): Key of the stream in Redis
        length (int): Number of events in the stream
        ttl (int): Expiration of the stream
    """

    key: str
    length: int
    ttl: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        key = self.key

        length = self.length

        ttl = self.ttl

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "key": key,
                "length": length,
                "ttl": ttl,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        key = d.pop("key")

        length = d.pop("length")

        ttl = d.pop("ttl")

        stream_info = cls(
            key=key,
            length=length,
            ttl=ttl,
        )

        stream_info.additional_properties = d
        return stream_info

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
