from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.add_event import AddEvent


T = TypeVar("T", bound="AddEventsRequest")


@_attrs_define
class AddEventsRequest:
    """
    Attributes:
        key (str): Key of the stream to write to
        events (list['AddEvent']): Events to add to the stream
    """

    key: str
    events: list["AddEvent"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        key = self.key

        events = []
        for events_item_data in self.events:
            events_item = events_item_data.to_dict()
            events.append(events_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "key": key,
                "events": events,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.add_event import AddEvent

        d = dict(src_dict)
        key = d.pop("key")

        events = []
        _events = d.pop("events")
        for events_item_data in _events:
            events_item = AddEvent.from_dict(events_item_data)

            events.append(events_item)

        add_events_request = cls(
            key=key,
            events=events,
        )

        add_events_request.additional_properties = d
        return add_events_request

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
