/* EI's two in-process demo stages share the IsOS v1 node ABI. */
#include "isos_node_v1.h"
#include <stdlib.h>

typedef struct {
  const isos_node_host_v1 *host;
  int ready;
} ei_demo_node;

static int32_t create(const uint8_t *config, size_t length,
                      const isos_node_host_v1 *host, isos_node_handle **out) {
  (void)config;
  (void)length;
  if (host == NULL || out == NULL || host->abi_version != 1 ||
      host->struct_size < sizeof(isos_node_host_v1) || host->emit == NULL) {
    return -1;
  }
  ei_demo_node *node = calloc(1, sizeof(*node));
  if (node == NULL) return -1;
  node->host = host;
  *out = (isos_node_handle *)node;
  return 0;
}

static int32_t start(isos_node_handle *handle) {
  if (handle == NULL) return -1;
  ((ei_demo_node *)handle)->ready = 1;
  return 0;
}

static int32_t input(isos_node_handle *handle, const uint8_t *data, size_t length) {
  ei_demo_node *node = (ei_demo_node *)handle;
  if (node == NULL || !node->ready || data == NULL || length > 1048576) return -1;
  return node->host->emit(node->host->context, data, length);
}

static int32_t health(isos_node_handle *handle) {
  if (handle == NULL) return -1;
  return ((ei_demo_node *)handle)->ready;
}

static int32_t stop(isos_node_handle *handle) {
  if (handle == NULL) return -1;
  ((ei_demo_node *)handle)->ready = 0;
  return 0;
}

static void destroy(isos_node_handle *handle) { free(handle); }

static const isos_node_api_v1 API = {1, sizeof(isos_node_api_v1), create,
                                     start, input, health, stop, destroy};
const isos_node_api_v1 *isos_node_v1(void) { return &API; }
