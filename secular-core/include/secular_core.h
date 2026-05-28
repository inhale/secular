/**
 * secular-core — C FFI header for Secular VPN core
 * 
 * This header provides the C interface for platforms that need
 * direct FFI access (iOS Swift, Android JNI via NDK).
 */

#ifndef SECULAR_CORE_H
#define SECULAR_CORE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Connection states */
#define SECULAR_STATE_DISCONNECTED 0
#define SECULAR_STATE_HANDSHAKING  1
#define SECULAR_STATE_CONNECTED    2
#define SECULAR_STATE_FAILED       3

/* Opaque handle type */
typedef struct SecularHandle SecularHandle;

/**
 * Create a new Secular engine from JSON configuration.
 * 
 * @param config_json  UTF-8 JSON configuration string
 * @param len          Length of the configuration string in bytes
 * @return             Opaque handle, or NULL on failure
 */
SecularHandle* secular_create(const uint8_t* config_json, size_t len);

/**
 * Destroy a Secular engine handle and free all resources.
 * 
 * @param handle  Handle returned by secular_create (may be NULL)
 */
void secular_destroy(SecularHandle* handle);

/**
 * Get the current connection state.
 * 
 * @param handle  Valid Secular handle
 * @return        State code (0-3), or -1 if handle is NULL
 */
int32_t secular_state(const SecularHandle* handle);

#ifdef __cplusplus
}
#endif

#endif /* SECULAR_CORE_H */
