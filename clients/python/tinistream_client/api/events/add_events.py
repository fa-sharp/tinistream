from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.add_events_request import AddEventsRequest
from ...models.add_events_response import AddEventsResponse
from ...models.error_message import ErrorMessage
from ...types import Response


def _get_kwargs(
    *,
    body: AddEventsRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/api/event/add",
    }

    _kwargs["json"] = body.to_dict()

    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[AddEventsResponse, ErrorMessage]]:
    if response.status_code == 200:
        response_200 = AddEventsResponse.from_dict(response.json())

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
) -> Response[Union[AddEventsResponse, ErrorMessage]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    *,
    client: AuthenticatedClient,
    body: AddEventsRequest,
) -> Response[Union[AddEventsResponse, ErrorMessage]]:
    """Add events

     Add events to a stream

    Args:
        body (AddEventsRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[AddEventsResponse, ErrorMessage]]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    *,
    client: AuthenticatedClient,
    body: AddEventsRequest,
) -> Optional[Union[AddEventsResponse, ErrorMessage]]:
    """Add events

     Add events to a stream

    Args:
        body (AddEventsRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[AddEventsResponse, ErrorMessage]
    """

    return sync_detailed(
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    body: AddEventsRequest,
) -> Response[Union[AddEventsResponse, ErrorMessage]]:
    """Add events

     Add events to a stream

    Args:
        body (AddEventsRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[AddEventsResponse, ErrorMessage]]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    *,
    client: AuthenticatedClient,
    body: AddEventsRequest,
) -> Optional[Union[AddEventsResponse, ErrorMessage]]:
    """Add events

     Add events to a stream

    Args:
        body (AddEventsRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[AddEventsResponse, ErrorMessage]
    """

    return (
        await asyncio_detailed(
            client=client,
            body=body,
        )
    ).parsed
