from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_message import ErrorMessage
from ...models.stream_info import StreamInfo
from ...types import UNSET, Response


def _get_kwargs(
    *,
    key: str,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    params["key"] = key

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/api/stream/info",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[ErrorMessage, StreamInfo]]:
    if response.status_code == 200:
        response_200 = StreamInfo.from_dict(response.json())

        return response_200

    if response.status_code == 400:
        response_400 = ErrorMessage.from_dict(response.json())

        return response_400

    if response.status_code == 401:
        response_401 = ErrorMessage.from_dict(response.json())

        return response_401

    if response.status_code == 404:
        response_404 = ErrorMessage.from_dict(response.json())

        return response_404

    if response.status_code == 422:
        response_422 = ErrorMessage.from_dict(response.json())

        return response_422

    if response.status_code == 500:
        response_500 = ErrorMessage.from_dict(response.json())

        return response_500

    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[ErrorMessage, StreamInfo]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    key: str,
) -> Response[Union[ErrorMessage, StreamInfo]]:
    """Get stream info

     Get info on an active stream

    Args:
        key (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[ErrorMessage, StreamInfo]]
    """

    kwargs = _get_kwargs(
        key=key,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    key: str,
) -> Optional[Union[ErrorMessage, StreamInfo]]:
    """Get stream info

     Get info on an active stream

    Args:
        key (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[ErrorMessage, StreamInfo]
    """

    return sync_detailed(
        client=client,
        key=key,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    key: str,
) -> Response[Union[ErrorMessage, StreamInfo]]:
    """Get stream info

     Get info on an active stream

    Args:
        key (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[ErrorMessage, StreamInfo]]
    """

    kwargs = _get_kwargs(
        key=key,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    key: str,
) -> Optional[Union[ErrorMessage, StreamInfo]]:
    """Get stream info

     Get info on an active stream

    Args:
        key (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[ErrorMessage, StreamInfo]
    """

    return (
        await asyncio_detailed(
            client=client,
            key=key,
        )
    ).parsed
