// secular-android/app/src/main/kotlin/com/secular/vpn/TunnelClient.kt
// TLS + HTTP/2 tunnel client using OkHttp for auth, then raw socket forwarding

package com.secular.vpn

import android.os.Build
import kotlinx.coroutines.*
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.InputStream
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.security.cert.X509Certificate
import java.util.concurrent.atomic.AtomicLong
import javax.net.ssl.*

class TunnelClient(
    private val serverHost: String,
    private val serverPort: Int,
    private val sniHostname: String,
    private val username: String,
    private val password: String
) {
    companion object {
        const val TAG = "TunnelClient"
    }

    private var tlsSocket: SSLSocket? = null
    private var inputStream: InputStream? = null
    private var outputStream: OutputStream? = null
    val bytesDownloaded = AtomicLong(0)
    val bytesUploaded = AtomicLong(0)

    suspend fun connect(): Boolean = withContext(Dispatchers.IO) {
        try {
            SecularVpnService.addLog("Tunnel: connecting to $serverHost:$serverPort (SNI=$sniHostname)")

            // Phase 1: TCP connect
            val tcpSocket = Socket()
            tcpSocket.connect(InetSocketAddress(serverHost, serverPort), 15000)
            tcpSocket.tcpNoDelay = true
            SecularVpnService.addLog("Tunnel: TCP connected")

            // Phase 2: TLS handshake with SNI + ALPN
            val sslContext = SSLContext.getInstance("TLS")
            val trustManager = object : X509TrustManager {
                override fun checkClientTrusted(chain: Array<out X509Certificate>?, authType: String?) {}
                override fun checkServerTrusted(chain: Array<out X509Certificate>?, authType: String?) {}
                override fun getAcceptedIssuers(): Array<X509Certificate> = arrayOf()
            }
            sslContext.init(null, arrayOf(trustManager), null)

            val sslSocketFactory = sslContext.socketFactory
            val sslSocket = sslSocketFactory.createSocket(tcpSocket, serverHost, serverPort, true) as SSLSocket

            // Enable SNI
            val sslParameters = sslSocket.sslParameters
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                sslParameters.serverNames = listOf(SNIHostName(sniHostname))
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                sslParameters.applicationProtocols = arrayOf("h2", "http/1.1")
            }
            sslSocket.sslParameters = sslParameters

            sslSocket.startHandshake()
            val alpn = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                sslSocket.applicationProtocol
            } else null
            SecularVpnService.addLog("Tunnel: TLS handshake complete, ALPN=$alpn")

            tlsSocket = sslSocket
            inputStream = sslSocket.inputStream
            outputStream = sslSocket.outputStream

            if (alpn != null && alpn != "h2") {
                SecularVpnService.addLog("Tunnel: WARNING ALPN=$alpn, expected h2")
                // Try anyway — some servers don't negotiate ALPN properly
            }

            // Phase 3: HTTP/2 auth request via OkHttp
            if (!doAuth()) {
                SecularVpnService.addLog("Tunnel: auth FAILED")
                return@withContext false
            }

            SecularVpnService.addLog("Tunnel: connected and authenticated")
            true
        } catch (e: Exception) {
            SecularVpnService.addLog("Tunnel: connect error: ${e.javaClass.simpleName}: ${e.message}")
            false
        }
    }

    private fun doAuth(): Boolean {
        return try {
            // Build auth body matching TrustTunnel protocol
            val authBody = JSONObject().apply {
                put("username", username)
                put("password", password)
                put("version", "0.2.0")
                put("platform", "android")
            }

            // Send HTTP/2 POST request directly on the TLS socket
            // This is a simplified HTTP/2 request — real HTTP/2 requires a frame encoder
            // For now, try HTTP/1.1 first (fallback that some servers support via ALPN)
            val request = buildString {
                append("POST /api/v1/auth HTTP/1.1\r\n")
                append("Host: $sniHostname\r\n")
                append("Content-Type: application/json\r\n")
                append("Content-Length: ${authBody.toString().toByteArray().size}\r\n")
                append("User-Agent: Secular/0.2.0\r\n")
                append("Connection: keep-alive\r\n")
                append("\r\n")
                append(authBody.toString())
            }

            SecularVpnService.addLog("Tunnel: sending auth POST to $sniHostname")
            outputStream?.write(request.toByteArray())
            outputStream?.flush()

            // Read response
            val response = ByteArray(4096)
            val bytesRead = inputStream?.read(response) ?: -1
            if (bytesRead > 0) {
                val responseStr = String(response, 0, bytesRead)
                SecularVpnService.addLog("Tunnel: auth response (${bytesRead} bytes): ${responseStr.take(200)}")
                // Check for 2xx status
                val statusLine = responseStr.lines().firstOrNull() ?: ""
                val status = statusLine.split(" ").getOrNull(1)?.toIntOrNull() ?: 0
                if (status in 200..299) {
                    SecularVpnService.addLog("Tunnel: auth OK (HTTP $status)")
                    true
                } else if (status == 401 || status == 403) {
                    SecularVpnService.addLog("Tunnel: auth rejected (HTTP $status)")
                    false
                } else {
                    SecularVpnService.addLog("Tunnel: unexpected auth status $status")
                    false
                }
            } else {
                SecularVpnService.addLog("Tunnel: auth read failed (bytesRead=$bytesRead)")
                false
            }
        } catch (e: Exception) {
            SecularVpnService.addLog("Tunnel: auth error: ${e.javaClass.simpleName}: ${e.message}")
            false
        }
    }

    fun getInputStream(): InputStream? = inputStream
    fun getOutputStream(): OutputStream? = outputStream

    fun disconnect() {
        try { inputStream?.close() } catch (_: Exception) {}
        try { outputStream?.close() } catch (_: Exception) {}
        try { tlsSocket?.close() } catch (_: Exception) {}
        tlsSocket = null
        inputStream = null
        outputStream = null
    }
}
