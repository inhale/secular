// secular-android/app/src/main/java/com/secular/vpn/data/DeepLinkParser.kt
// Parse tt:// deep-link URIs into ServerProfile
// Supports 3 formats (same as macOS config.py):
//   1. Base64-encoded TOML
//   2. TrustTunnel native binary TLV
//   3. URL-encoded key=value pairs

package com.secular.vpn.data

import android.net.Uri
import android.util.Base64

object DeepLinkParser {

    fun parse(uriString: String): ServerProfile? {
        if (!uriString.startsWith("tt://?")) return null
        val qs = uriString.removePrefix("tt://?")

        // Format 1: base64-encoded TOML
        parseBase64Toml(qs)?.let { return it }

        // Format 2: TrustTunnel native binary TLV
        parseTlv(qs)?.let { return it }

        // Format 3: URL-encoded key=value
        parseUrlEncoded(qs)?.let { return it }

        return null
    }

    private fun parseBase64Toml(qs: String): ServerProfile? {
        return try {
            val padded = qs + "=".repeat((-qs.length % 4).let { if (it == 0) 0 else it })
            val tomlStr = String(Base64.decode(padded, Base64.DEFAULT))
            // Simple TOML parsing - extract key=value pairs
            val fields = parseTomlFields(tomlStr)
            buildProfileFromFields(fields, tomlStr)
        } catch (e: Exception) {
            null
        }
    }

    private fun parseTomlFields(toml: String): Map<String, String> {
        val fields = mutableMapOf<String, String>()
        val lines = toml.lines()
        var currentSection = ""

        for (line in lines) {
            val trimmed = line.trim()
            if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
                currentSection = trimmed.substring(1, trimmed.length - 1)
                continue
            }
            val eqIdx = trimmed.indexOf('=')
            if (eqIdx > 0) {
                val key = trimmed.substring(0, eqIdx).trim()
                val value = trimmed.substring(eqIdx + 1).trim()
                    .trim('"').trim('\'')
                val fullKey = if (currentSection.isNotEmpty()) "$currentSection.$key" else key
                fields[fullKey] = value
                // Also store without section for convenience
                fields[key] = value
            }
        }
        return fields
    }

    private fun buildProfileFromFields(fields: Map<String, String>, raw: String): ServerProfile? {
        val hostname = fields["hostname"] ?: fields["endpoint.hostname"] ?: return null
        val addresses = parseAddresses(fields["addresses"] ?: fields["endpoint.addresses"])
        val username = fields["username"] ?: fields["endpoint.username"] ?: ""
        val password = fields["password"] ?: fields["endpoint.password"] ?: ""
        val name = fields["name"] ?: hostname
        val hasIpv6 = parseBoolean(fields["has_ipv6"] ?: fields["endpoint.has_ipv6"], true)
        val clientRandom = fields["client_random"] ?: fields["client_random_prefix"]
            ?: fields["endpoint.client_random"] ?: ""
        val skipVerification = parseBoolean(
            fields["skip_verification"] ?: fields["endpoint.skip_verification"], false
        )
        val certificate = fields["certificate"] ?: fields["endpoint.certificate"] ?: ""
        val upstreamProtocol = fields["upstream_protocol"]
            ?: fields["endpoint.upstream_protocol"] ?: "http2"
        val antiDpi = parseBoolean(fields["anti_dpi"] ?: fields["endpoint.anti_dpi"], false)
        val dnsUpstreams = parseDnsUpstreams(fields["dns_upstreams"]
            ?: fields["endpoint.dns_upstreams"])

        return ServerProfile(
            name = name,
            hostname = hostname,
            addresses = addresses,
            username = username,
            password = password,
            hasIpv6 = hasIpv6,
            clientRandom = clientRandom,
            skipVerification = skipVerification,
            certificate = certificate,
            upstreamProtocol = upstreamProtocol,
            antiDpi = antiDpi,
            dnsUpstreams = dnsUpstreams
        )
    }

    private fun parseAddresses(raw: String): List<String> {
        if (raw.isEmpty()) return emptyList()
        // Handle both comma-separated and TOML array syntax
        val cleaned = raw.removePrefix("[").removeSuffix("]")
            .replace("\"", "").replace("'", "")
        return cleaned.split(",").map { it.trim() }.filter { it.isNotEmpty() }
    }

    private fun parseDnsUpstreams(raw: String): List<String> {
        if (raw.isEmpty()) return emptyList()
        val cleaned = raw.removePrefix("[").removeSuffix("]")
            .replace("\"", "").replace("'", "")
        return cleaned.split(",").map { it.trim() }.filter { it.isNotEmpty() }
    }

    private fun parseBoolean(value: String?, default: Boolean): Boolean {
        if (value == null) return default
        return value.lowercase() == "true" || value == "1"
    }

    private fun parseTlv(qs: String): ServerProfile? {
        return try {
            val padded = qs + "=".repeat((-qs.length % 4).let { if (it == 0) 0 else it })
            val data = Base64.decode(padded, Base64.DEFAULT)
            if (data.size < 5) return null

            // Skip 4-byte header, first field is implicit hostname
            var pos = 4
            if (pos >= data.size) return null
            val hostnameLen = data[pos].toInt() and 0xFF
            pos += 1
            if (pos + hostnameLen > data.size) return null
            val hostname = String(data, pos, hostnameLen)
            pos += hostnameLen

            // Build field map: tag 9 = hostname
            val fields = mutableMapOf<Int, String>()
            fields[9] = hostname

            // Parse remaining TLV fields: [tag:1][len:1][value:N]
            while (pos + 2 <= data.size) {
                val tag = data[pos].toInt() and 0xFF
                val vlen = data[pos + 1].toInt() and 0xFF
                pos += 2
                if (pos + vlen > data.size) break
                val value = String(data, pos, vlen).replace("\u0000", "")
                fields[tag] = value
                pos += vlen
            }

            // tag mapping: 2=address, 5=username, 6=password, 7=client_random
            if (fields[5].isNullOrEmpty() || fields[6].isNullOrEmpty()) return null

            ServerProfile(
                name = fields[9] ?: "TrustTunnel Server",
                hostname = fields[9] ?: "",
                addresses = fields[2]?.split(",")?.map { it.trim() }?.filter { it.isNotEmpty() }
                    ?: emptyList(),
                username = fields[5] ?: "",
                password = fields[6] ?: "",
                clientRandom = fields[7] ?: "",
                skipVerification = true,
                certificate = "",
                upstreamProtocol = "http2",
                dnsUpstreams = emptyList()
            )
        } catch (e: Exception) {
            null
        }
    }

    private fun parseUrlEncoded(qs: String): ServerProfile? {
        return try {
            val uri = Uri.parse("http://localhost?$qs")
            val hostname = uri.getQueryParameter("hostname") ?: return null
            val addressesRaw = uri.getQueryParameter("addresses") ?: ""
            val addresses = addressesRaw.split(",").map { it.trim() }.filter { it.isNotEmpty() }
            val name = uri.getQueryParameter("name") ?: hostname
            val username = uri.getQueryParameter("username") ?: ""
            val password = uri.getQueryParameter("password") ?: ""
            val clientRandom = uri.getQueryParameter("client_random") ?: ""
            val certificate = uri.getQueryParameter("certificate") ?: ""
            val skipVerification = uri.getQueryParameter("skip_verification").let {
                it?.lowercase() == "true"
            }
            val upstreamProtocol = uri.getQueryParameter("upstream_protocol") ?: "http2"
            val antiDpi = uri.getQueryParameter("anti_dpi").let { it?.lowercase() == "true" }
            val dnsUpstreams = uri.getQueryParameters("dns_upstream")
            val hasIpv6 = uri.getQueryParameter("has_ipv6")?.lowercase() != "false"

            ServerProfile(
                name = name,
                hostname = hostname,
                addresses = addresses,
                username = username,
                password = password,
                hasIpv6 = hasIpv6,
                clientRandom = clientRandom,
                skipVerification = skipVerification,
                certificate = certificate,
                upstreamProtocol = upstreamProtocol,
                antiDpi = antiDpi,
                dnsUpstreams = dnsUpstreams.ifEmpty { emptyList() }
            )
        } catch (e: Exception) {
            null
        }
    }
}
