// secular-android/app/src/main/kotlin/com/adguard/trusttunnel/DeepLink.kt
// JNI bridge for native DeepLink.decode() used to parse tt:// URIs

package com.adguard.trusttunnel

object DeepLink {
    init {
        System.loadLibrary("trusttunnel_android")
    }

    /**
     * Decode a `tt://` deep-link URI into a `[endpoint]` TOML section string.
     * Maps to Java_com_adguard_trusttunnel_DeepLink_decode JNI method.
     *
     * @param uri The `tt://...` deep-link URI to decode
     * @return A TOML string beginning with [endpoint] for embedding in the full config
     * @throws RuntimeException if the URI is invalid or missing required fields
     */
    @JvmStatic
    external fun decode(uri: String): String
}
