from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="RedisStats")


@_attrs_define
class RedisStats:
    """
    Attributes:
        static (int): Number of static connections
        streaming (int): Number of streaming connections
        streaming_in_use (int): Number of in-use streaming connections
        streaming_available (int): Number of available streaming connections
        streaming_max (int): Maximum number of streaming connections
    """

    static: int
    streaming: int
    streaming_in_use: int
    streaming_available: int
    streaming_max: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        static = self.static

        streaming = self.streaming

        streaming_in_use = self.streaming_in_use

        streaming_available = self.streaming_available

        streaming_max = self.streaming_max

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "static": static,
                "streaming": streaming,
                "streaming_in_use": streaming_in_use,
                "streaming_available": streaming_available,
                "streaming_max": streaming_max,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        static = d.pop("static")

        streaming = d.pop("streaming")

        streaming_in_use = d.pop("streaming_in_use")

        streaming_available = d.pop("streaming_available")

        streaming_max = d.pop("streaming_max")

        redis_stats = cls(
            static=static,
            streaming=streaming,
            streaming_in_use=streaming_in_use,
            streaming_available=streaming_available,
            streaming_max=streaming_max,
        )

        redis_stats.additional_properties = d
        return redis_stats

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
