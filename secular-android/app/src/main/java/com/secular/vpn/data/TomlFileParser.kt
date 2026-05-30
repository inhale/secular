// secular-android/app/src/main/java/com/secular/vpn/data/TomlFileParser.kt
// Parse TrustTunnel TOML config files

package com.secular.vpn.data

import java.io.InputStream

object TomlFileParser {

    fun parse(inputStream: InputStream): ServerProfile? {
        return try {
            val tomlStr = inputStream.bufferedReader().readText()
            val fields = parseTomlFields(tomlStr)

            val hostname = fields["hostname"] ?: fields["endpoint.hostname"] ?: return null
            val addresses = parseAddresses(fields["addresses"] ?: fields["endpoint.addresses"] ?: "")
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
                ?: fields["endpoint.dns_upstreams"] ?: "")

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
                dnsUpstreams = dnsUpstreams
            )
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
            if (trimmed.isEmpty() || trimmed.startsWith("#")) continue
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
                fields[key] = value
            }
        }
        return fields
    }

    private fun parseAddresses(raw: String): List<String> {
        if (raw.isEmpty()) return emptyList()
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
}
