from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.error_message import ErrorMessage
from ...models.stream_info import StreamInfo
from ...types import UNSET, Response, Unset


def _get_kwargs(
    *,
    pattern: Union[None, Unset, str] = UNSET,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    json_pattern: Union[None, Unset, str]
    if isinstance(pattern, Unset):
        json_pattern = UNSET
    else:
        json_pattern = pattern
    params["pattern"] = json_pattern

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": "/api/stream/",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[ErrorMessage, list["StreamInfo"]]]:
    if response.status_code == 200:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in _response_200:
            response_200_item = StreamInfo.from_dict(response_200_item_data)

            response_200.append(response_200_item)

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
) -> Response[Union[ErrorMessage, list["StreamInfo"]]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    pattern: Union[None, Unset, str] = UNSET,
) -> Response[Union[ErrorMessage, list["StreamInfo"]]]:
    """List streams

     List all active streams

    Args:
        pattern (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[ErrorMessage, list['StreamInfo']]]
    """

    kwargs = _get_kwargs(
        pattern=pattern,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    pattern: Union[None, Unset, str] = UNSET,
) -> Optional[Union[ErrorMessage, list["StreamInfo"]]]:
    """List streams

     List all active streams

    Args:
        pattern (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[ErrorMessage, list['StreamInfo']]
    """

    return sync_detailed(
        client=client,
        pattern=pattern,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    pattern: Union[None, Unset, str] = UNSET,
) -> Response[Union[ErrorMessage, list["StreamInfo"]]]:
    """List streams

     List all active streams

    Args:
        pattern (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[ErrorMessage, list['StreamInfo']]]
    """

    kwargs = _get_kwargs(
        pattern=pattern,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    pattern: Union[None, Unset, str] = UNSET,
) -> Optional[Union[ErrorMessage, list["StreamInfo"]]]:
    """List streams

     List all active streams

    Args:
        pattern (Union[None, Unset, str]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[ErrorMessage, list['StreamInfo']]
    """

    return (
        await asyncio_detailed(
            client=client,
            pattern=pattern,
        )
    ).parsed
