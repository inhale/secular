// secular-android/app/src/main/java/com/secular/vpn/data/DeepLinkParser.kt
// Parse tt:// deep-link URIs into ServerProfile
// Supports 3 formats:
//   1. Base64-encoded TOML
//   2. TrustTunnel native binary TLV
//   3. URL-encoded key=value pairs

package com.secular.vpn.data

import android.net.Uri
import android.util.Base64

object DeepLinkParser {

    fun parse(uriString: String): ServerProfile? {
        val trimmed = uriString.trim()
        if (trimmed.isEmpty()) return null
        android.util.Log.d("DeepLinkParser", "Parsing: ${trimmed.take(80)}")

        val qs: String
        when {
            trimmed.startsWith("tt://?") -> qs = trimmed.removePrefix("tt://?")
            trimmed.startsWith("tt://") -> qs = trimmed.removePrefix("tt://")
            trimmed.startsWith("tl://") -> qs = trimmed.removePrefix("tl://")
            else -> return null
        }

        if (qs.isEmpty()) return null
        android.util.Log.d("DeepLinkParser", "qs length=${qs.length}, qs=${qs.take(80)}")

    // Try all 3 formats in order
        val r1 = parseBase64Toml(qs)
        if (r1 != null) return r1
        android.util.Log.d("DeepLinkParser", "base64 TOML: no match")
        val r2 = parseTlv(qs)
        if (r2 != null) return r2
        android.util.Log.d("DeepLinkParser", "TLV: no match")
        val r3 = parseUrlEncoded(qs)
        if (r3 != null) return r3
        android.util.Log.d("DeepLinkParser", "URL-encoded: no match")

        return null
    }

    // ── Format 1: base64-encoded TOML ──
    private fun parseBase64Toml(qs: String): ServerProfile? {
        return try {
            val padLen = (4 - qs.length % 4) % 4
            val padded = qs + "=".repeat(padLen)
            val tomlStr = String(Base64.decode(padded, Base64.DEFAULT))
            val fields = parseTomlFields(tomlStr)
            buildProfileFromFields(fields)
        } catch (_: Exception) { null }
    }

    private fun parseTomlFields(toml: String): Map<String, String> {
        val fields = mutableMapOf<String, String>()
        var currentSection = ""
        for (line in toml.lines()) {
            val t = line.trim()
            if (t.isEmpty() || t.startsWith("#")) continue
            if (t.startsWith("[") && t.endsWith("]")) {
                currentSection = t.substring(1, t.length - 1)
                continue
            }
            val eq = t.indexOf('=')
            if (eq > 0) {
                val key = t.substring(0, eq).trim()
                val value = t.substring(eq + 1).trim().trim('"').trim('\'')
                val fullKey = if (currentSection.isNotEmpty()) "$currentSection.$key" else key
                fields[fullKey] = value
                fields[key] = value // also store without section prefix
            }
        }
        return fields
    }

    private fun buildProfileFromFields(fields: Map<String, String>): ServerProfile? {
        val hostname = fields["hostname"]
            ?: fields["endpoint.hostname"] ?: fields["endpoints.hostname"]
            ?: return null
        val addresses = parseList(
            fields["addresses"] ?: fields["endpoint.addresses"]
                ?: fields["endpoints.addresses"] ?: ""
        )
        val username = fields["username"] ?: fields["endpoint.username"]
            ?: fields["endpoints.username"] ?: ""
        val password = fields["password"] ?: fields["endpoint.password"]
            ?: fields["endpoints.password"] ?: ""
        val name = fields["name"] ?: hostname
        return ServerProfile(
            name = name, hostname = hostname, addresses = addresses,
            username = username, password = password,
            hasIpv6 = parseBool(
                fields["has_ipv6"] ?: fields["endpoint.has_ipv6"]
                    ?: fields["endpoints.has_ipv6"], true
            ),
            clientRandom = fields["client_random"] ?: fields["client_random_prefix"]
                ?: fields["endpoint.client_random"]
                ?: fields["endpoints.client_random"] ?: "",
            skipVerification = parseBool(
                fields["skip_verification"] ?: fields["endpoint.skip_verification"]
                    ?: fields["endpoints.skip_verification"], false
            ),
            certificate = fields["certificate"] ?: fields["endpoint.certificate"]
                ?: fields["endpoints.certificate"] ?: "",
            upstreamProtocol = fields["upstream_protocol"]
                ?: fields["endpoint.upstream_protocol"]
                ?: fields["endpoints.upstream_protocol"] ?: "http2",
            antiDpi = parseBool(
                fields["anti_dpi"] ?: fields["endpoint.anti_dpi"]
                    ?: fields["endpoints.anti_dpi"], false
            ),
            dnsUpstreams = parseList(
                fields["dns_upstreams"] ?: fields["endpoint.dns_upstreams"]
                    ?: fields["endpoints.dns_upstreams"] ?: ""
            )
        )
    }

    // ── Format 2: TrustTunnel binary TLV ──
    // Wire format: sequence of [tag:1][len:1][value:N]
    // Tag 0x00 = version byte (1 byte value)
    // Tag 0x01 = name/hostname
    // Tag 0x02 = address (ip:port)
    // Tag 0x05 = username
    // Tag 0x06 = password
    // Tag 0x08+ = certificate PEM chunks (binary) — concatenated into certificate field
    // Tag 0x0c = display name suffix
    // ── Format 2: TrustTunnel binary TLV ──
    private fun parseTlv(qs: String): ServerProfile? {
        return try {
            val padLen = (4 - qs.length % 4) % 4
            val padded = qs + "=".repeat(padLen)
            val data = Base64.decode(padded, Base64.DEFAULT)
            if (data.size < 4) return null

            val fields = mutableMapOf<Int, ByteArray>()
            var pos = 0
            while (pos + 2 <= data.size) {
                val tag = data[pos].toInt() and 0xFF
                val vlen = data[pos + 1].toInt() and 0xFF
                pos += 2
                if (pos + vlen > data.size) break
                fields[tag] = data.copyOfRange(pos, pos + vlen)
                pos += vlen
            }

            val nameBytes = fields[0x01] ?: return null
            val name = String(nameBytes, Charsets.UTF_8).replace("\u0000", "")
            if (name.isEmpty()) return null

            val suffixBytes = fields[0x0c]
            val suffix = if (suffixBytes != null) String(suffixBytes, Charsets.UTF_8).replace("\u0000", "") else ""
            val displayName = if (suffix.isNotEmpty()) "$name $suffix" else name

            val addressBytes = fields[0x02] ?: return null
            val address = String(addressBytes, Charsets.UTF_8).replace("\u0000", "")
            if (address.isEmpty()) return null

            val usernameBytes = fields[0x05]
            val username = String(usernameBytes, Charsets.UTF_8).replace("\u0000", "")

            val passwordBytes = fields[0x06]
            val password = String(passwordBytes, Charsets.UTF_8).replace("\u0000", "")

            val certBuilder = StringBuilder()
            for ((tag, bytes) in fields) {
                if (tag >= 0x08 && tag != 0x0c) {
                    try {
                        val s = String(bytes, Charsets.UTF_8)
                        if (s.contains("-----BEGIN") || s.contains("MII")) {
                            certBuilder.append(s.replace("\u0000", ""))
                        }
                    } catch (_: Exception) { }
                }
            }

            ServerProfile(
                name = displayName, hostname = name, addresses = listOf(address),
                username = username, password = password,
                hasIpv6 = true, skipVerification = true,
                certificate = certBuilder.toString(),
                upstreamProtocol = "http2", dnsUpstreams = emptyList()
            )
        } catch (_: Exception) { null }
    }

    // ── Format 3: URL-encoded key=value ──
    private fun parseUrlEncoded(qs: String): ServerProfile? {
        return try {
            val uri = Uri.parse("http://localhost?$qs")
            val hostname = uri.getQueryParameter("hostname") ?: return null
        val addresses = parseList(uri.getQueryParameter("addresses") ?: "")
        val name = uri.getQueryParameter("name") ?: hostname
        ServerProfile(
            name = name, hostname = hostname, addresses = addresses,
            username = uri.getQueryParameter("username") ?: "",
            password = uri.getQueryParameter("password") ?: "",
            hasIpv6 = uri.getQueryParameter("has_ipv6")?.lowercase() != "false",
            clientRandom = uri.getQueryParameter("client_random") ?: "",
            certificate = uri.getQueryParameter("certificate") ?: "",
            skipVerification = uri.getQueryParameter("skip_verification")?.lowercase() == "true",
            upstreamProtocol = uri.getQueryParameter("upstream_protocol") ?: "http2",
            antiDpi = uri.getQueryParameter("anti_dpi")?.lowercase() == "true",
            dnsUpstreams = uri.getQueryParameters("dns_upstream").ifEmpty { emptyList() }
            )
        } catch (_: Exception) { null }
    }

    // ── Helpers ──
    private fun parseList(raw: String): List<String> {
        if (raw.isEmpty()) return emptyList()
        return raw.removePrefix("[").removeSuffix("]")
            .replace("\"", "").replace("'", "")
            .split(",").map { it.trim() }.filter { it.isNotEmpty() }
    }

    private fun parseBool(value: String?, default: Boolean): Boolean {
        if (value == null) return default
        return value.lowercase() == "true" || value == "1"
    }
}
