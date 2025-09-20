from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.add_events_stream_response import AddEventsStreamResponse
from ...models.error_message import ErrorMessage
from ...types import UNSET, File, Response


def _get_kwargs(
    *,
    body: File,
    key: str,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    params: dict[str, Any] = {}

    params["key"] = key

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/api/event/add/json-stream",
        "params": params,
    }

    _kwargs["content"] = body.payload

    headers["Content-Type"] = "application/octet-stream"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[AddEventsStreamResponse, ErrorMessage]]:
    if response.status_code == 200:
        response_200 = AddEventsStreamResponse.from_dict(response.json())

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
) -> Response[Union[AddEventsStreamResponse, ErrorMessage]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    body: File,
    key: str,
) -> Response[Union[AddEventsStreamResponse, ErrorMessage]]:
    """Add events JSON stream

     Add events to a stream via a JSON stream. Events are sent as newline-delimited JSON objects.

    Args:
        key (str):
        body (File):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[AddEventsStreamResponse, ErrorMessage]]
    """

    kwargs = _get_kwargs(
        body=body,
        key=key,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    body: File,
    key: str,
) -> Optional[Union[AddEventsStreamResponse, ErrorMessage]]:
    """Add events JSON stream

     Add events to a stream via a JSON stream. Events are sent as newline-delimited JSON objects.

    Args:
        key (str):
        body (File):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[AddEventsStreamResponse, ErrorMessage]
    """

    return sync_detailed(
        client=client,
        body=body,
        key=key,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    body: File,
    key: str,
) -> Response[Union[AddEventsStreamResponse, ErrorMessage]]:
    """Add events JSON stream

     Add events to a stream via a JSON stream. Events are sent as newline-delimited JSON objects.

    Args:
        key (str):
        body (File):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[AddEventsStreamResponse, ErrorMessage]]
    """

    kwargs = _get_kwargs(
        body=body,
        key=key,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    body: File,
    key: str,
) -> Optional[Union[AddEventsStreamResponse, ErrorMessage]]:
    """Add events JSON stream

     Add events to a stream via a JSON stream. Events are sent as newline-delimited JSON objects.

    Args:
        key (str):
        body (File):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[AddEventsStreamResponse, ErrorMessage]
    """

    return (
        await asyncio_detailed(
            client=client,
            body=body,
            key=key,
        )
    ).parsed
