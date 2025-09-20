from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.redis_stats import RedisStats


T = TypeVar("T", bound="InfoResponse")


@_attrs_define
class InfoResponse:
    """
    Attributes:
        url (str):
        version (str):
        redis (RedisStats):
    """

    url: str
    version: str
    redis: "RedisStats"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        url = self.url

        version = self.version

        redis = self.redis.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "url": url,
                "version": version,
                "redis": redis,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.redis_stats import RedisStats

        d = dict(src_dict)
        url = d.pop("url")

        version = d.pop("version")

        redis = RedisStats.from_dict(d.pop("redis"))

        info_response = cls(
            url=url,
            version=version,
            redis=redis,
        )

        info_response.additional_properties = d
        return info_response

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
