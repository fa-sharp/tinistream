from enum import Enum


class StreamStatus(str, Enum):
    ACTIVE = "active"
    CANCELLED = "cancelled"
    ENDED = "ended"

    def __str__(self) -> str:
        return str(self.value)
