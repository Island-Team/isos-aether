"""Reusable EOS gRPC channel lifecycle for core link clients."""

from __future__ import annotations

import grpc

from ei.lib.config import GatewayConfig


class GatewayTransport:
  """Hold one validated EOS endpoint and lazily opened gRPC channel."""

  def __init__(self, gateway: GatewayConfig) -> None:
    self.target = gateway.endpoint
    self._channel: grpc.aio.Channel | None = None

  def open(self) -> grpc.aio.Channel:
    """Return the active channel, creating it only on first use."""
    if self._channel is None:
      self._channel = grpc.aio.insecure_channel(self.target)
    return self._channel

  @property
  def is_open(self) -> bool:
    return self._channel is not None

  async def aclose(self) -> None:
    """Close once; a later use may open a fresh channel."""
    channel = self._channel
    self._channel = None
    if channel is not None:
      await channel.close()
