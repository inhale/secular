// secular-android/app/src/main/java/com/secular/vpn/data/ServerProfile.kt
// Server profile data model matching TrustTunnel TOML config

package com.secular.vpn.data

import com.google.gson.annotations.SerializedName

data class ServerProfile(
    @SerializedName("name") val name: String = "",
    @SerializedName("hostname") val hostname: String = "",
    @SerializedName("addresses") val addresses: List<String> = emptyList(),  // "ip:port" format
    @SerializedName("username") val username: String = "",
    @SerializedName("password") val password: String = "",
    @SerializedName("has_ipv6") val hasIpv6: Boolean = true,
    @SerializedName("client_random") val clientRandom: String = "",
    @SerializedName("skip_verification") val skipVerification: Boolean = false,
    @SerializedName("certificate") val certificate: String = "",  // path to .pem file
    @SerializedName("upstream_protocol") val upstreamProtocol: String = "http2",  // "http2" | "http3"
    @SerializedName("anti_dpi") val antiDpi: Boolean = false,
    @SerializedName("dns_upstreams") val dnsUpstreams: List<String> = emptyList(),
    @SerializedName("bypass_domains") val bypassDomains: List<String> = emptyList(), // domains/IPs excluded from tunnel
) {
    val ipAddress: String
        get() = addresses.firstOrNull()?.substringBefore(":") ?: ""

    val displayAddress: String
        get() = addresses.firstOrNull() ?: ""

    val protocolDisplay: String
        get() = if (upstreamProtocol == "http3") "QUIC" else "HTTP/2"

    companion object {
        fun empty(name: String = "") = ServerProfile(name = name)
    }
}
