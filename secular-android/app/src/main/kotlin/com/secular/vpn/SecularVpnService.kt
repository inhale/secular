// Android VpnService implementation
package com.secular.vpn

import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import android.util.Log
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.*
import java.io.InputStream
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.util.concurrent.atomic.AtomicLong

class SecularVpnService : VpnService() {
    companion object {
        const val TAG = "SecularVPN"
        const val ACTION_CONNECT = "com.secular.vpn.CONNECT"
        const val ACTION_DISCONNECT = "com.secular.vpn.DISCONNECT"

        // Observable state for UI
        @Volatile var isTunnelUp = false
            private set
        @Volatile var lastError: String? = null
            private set
        val bytesDownloaded = AtomicLong(0)
        val bytesUploaded = AtomicLong(0)
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var job: Job? = null
    private var serverSocket: Socket? = null

    private data class ServerConfig(
        @SerializedName("hostname") val hostname: String = "",
        @SerializedName("addresses") val addresses: List<String> = emptyList(),
        @SerializedName("username") val username: String = "",
        @SerializedName("password") val password: String = "",
        @SerializedName("has_ipv6") val hasIpv6: Boolean = true,
        @SerializedName("upstream_protocol") val upstreamProtocol: String = "http2",
        @SerializedName("dns_upstreams") val dnsUpstreams: List<String> = emptyList(),
        @SerializedName("certificate") val certificate: String = "",
        @SerializedName("skip_verification") val skipVerification: Boolean = false,
        @SerializedName("anti_dpi") val antiDpi: Boolean = false,
        @SerializedName("client_random") val clientRandom: String = ""
    )

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        return when (intent?.action) {
            ACTION_CONNECT -> {
                val serverJson = intent.getStringExtra("server_json")
                connect(serverJson)
                START_STICKY
            }
            ACTION_DISCONNECT -> {
                disconnect()
                START_NOT_STICKY
            }
            else -> START_NOT_STICKY
        }
    }

    private fun connect(serverJson: String?) {
        Log.d(TAG, "Connecting... server=${serverJson != null}")

        // Parse server config
        val config = try {
            if (serverJson != null) Gson().fromJson(serverJson, ServerConfig::class.java) else null
        } catch (e: Exception) {
            Log.e(TAG, "Failed to parse server config: ${e.message}")
            lastError = "Invalid server config"
            return
        }

        if (config == null) {
            lastError = "No server config provided"
            return
        }

        val address = config.addresses.firstOrNull()
        if (address.isNullOrEmpty()) {
            lastError = "No server address"
            return
        }

        // Build VPN interface
        val builder = Builder()
            .setSession("Secular")
            .setMtu(1380)
            .addAddress("10.0.0.2", 32)
            .addRoute("0.0.0.0", 0)
            .setBlocking(true)

        // DNS
        if (config.dnsUpstreams.isNotEmpty()) {
            config.dnsUpstreams.forEach { dns ->
                try {
                    builder.addDnsServer(java.net.InetAddress.getByName(dns))
                } catch (e: Exception) {
                    Log.e(TAG, "Invalid DNS: $dns")
                }
            }
        } else {
            builder.addDnsServer(java.net.InetAddress.getByName("9.9.9.9"))
        }

        vpnInterface = builder.establish()
        if (vpnInterface == null) {
            Log.e(TAG, "Failed to establish VPN interface")
            lastError = "VPN permission denied"
            return
        }

        Log.d(TAG, "VPN interface established: fd=${vpnInterface!!.fd}")

        // Connect to remote server
        connectToServer(config, address)
    }

    private fun connectToServer(config: ServerConfig, address: String) {
        val host = address.substringBefore(":")
        val port = address.substringAfter(":", "443").toIntOrNull() ?: 443

        Log.d(TAG, "Connecting to $host:$port hostname=${config.hostname}")

        job = CoroutineScope(Dispatchers.IO).launch {
            try {
                val socket = Socket()
                socket.connect(InetSocketAddress(host, port), 15000)
                socket.tcpNoDelay = true
                serverSocket = socket
                Log.d(TAG, "TCP connected to $host:${socket.port}")
                isTunnelUp = true
                lastError = null
                bytesDownloaded.set(0)
                bytesUploaded.set(0)

                // Forward packets between VPN interface and server socket
                val vpn = vpnInterface ?: run {
                    lastError = "VPN interface lost"
                    isTunnelUp = false
                    return@launch
                }
                forwardPackets(vpn, socket)
            } catch (e: Exception) {
                Log.e(TAG, "Server connection failed: ${e.message}")
                lastError = "Connection failed: ${e.message}"
                isTunnelUp = false
            }
        }
    }

    private suspend fun forwardPackets(vpn: ParcelFileDescriptor, socket: Socket) {
        Log.d(TAG, "Packet forwarding started: fd=${vpn.fd}")

        val vpnInput = ParcelFileDescriptor.AutoCloseInputStream(vpn)
        val vpnOutput = ParcelFileDescriptor.AutoCloseOutputStream(vpn)
        val sockInput: InputStream = socket.getInputStream()
        val sockOutput: OutputStream = socket.getOutputStream()

        // VPN -> Server (upload)
        val vpnToServer = CoroutineScope(Dispatchers.IO).launch {
            try {
                val buffer = ByteArray(32767)
                while (isActive && !socket.isClosed) {
                    val length = vpnInput.read(buffer)
                    if (length > 0) {
                        sockOutput.write(buffer, 0, length)
                        sockOutput.flush()
                        bytesUploaded.addAndGet(length.toLong())
                    } else if (length < 0) break
                }
            } catch (e: Exception) {
                Log.e(TAG, "VPN→Server error: ${e.message}")
            }
        }

        // Server -> VPN (download)
        val serverToVpn = CoroutineScope(Dispatchers.IO).launch {
            try {
                val buffer = ByteArray(32767)
                while (isActive && !socket.isClosed) {
                    val length = sockInput.read(buffer)
                    if (length > 0) {
                        vpnOutput.write(buffer, 0, length)
                        vpnOutput.flush()
                        bytesDownloaded.addAndGet(length.toLong())
                    } else if (length < 0) break
                }
            } catch (e: Exception) {
                Log.e(TAG, "Server→VPN error: ${e.message}")
            }
        }

        joinAll(vpnToServer, serverToVpn)
        Log.d(TAG, "Packet forwarding ended")
        isTunnelUp = false
    }

    private fun disconnect() {
        Log.d(TAG, "Disconnecting...")
        isTunnelUp = false
        lastError = null
        job?.cancel()
        try { serverSocket?.close() } catch (_: Exception) {}
        serverSocket = null
        try { vpnInterface?.close() } catch (_: Exception) {}
        vpnInterface = null
        bytesDownloaded.set(0)
        bytesUploaded.set(0)
    }

    override fun onDestroy() {
        disconnect()
        super.onDestroy()
    }
}
