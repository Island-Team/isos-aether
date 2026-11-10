"""EOS core link transport lifecycle and endpoint validation."""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from ei.core.link.transport import GatewayTransport
from ei.lib.config import GatewayConfig


@pytest.mark.asyncio
async def test_tcp_channel_is_lazy_reused_and_closed() -> None:
  transport = GatewayTransport(GatewayConfig(host="127.0.0.1", grpc_port=16200))
  assert transport.target == "127.0.0.1:16200"
  assert not transport.is_open
  first = transport.open()
  assert transport.open() is first
  await transport.aclose()
  assert not transport.is_open
  await transport.aclose()
  second = transport.open()
  assert second is not first
  await transport.aclose()


@pytest.mark.parametrize(
  "endpoint",
  [
    {"transport": "tcp"},
    {"transport": "http", "host": "127.0.0.1"},
    {"transport": "uds"},
  ],
)
def test_invalid_or_missing_endpoint_is_rejected(endpoint: dict[str, str]) -> None:
  with pytest.raises(ValidationError):
    GatewayTransport(GatewayConfig.model_validate(endpoint))
