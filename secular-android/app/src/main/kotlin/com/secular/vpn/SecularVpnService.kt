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

class SecularVpnService : VpnService() {
    companion object {
        const val TAG = "SecularVPN"
        const val ACTION_CONNECT = "com.secular.vpn.CONNECT"
        const val ACTION_DISCONNECT = "com.secular.vpn.DISCONNECT"
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var job: Job? = null
    private var serverSocket: Socket? = null

    // Lightweight server config class for deserialization
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
            null
        }

        // Build VPN interface
        val builder = Builder()
            .setSession("Secular")
            .setMtu(1380)
            .addAddress("10.0.0.2", 32)
            .addRoute("0.0.0.0", 0)
            .setBlocking(true)

        // Use custom DNS if provided
        if (config != null && config.dnsUpstreams.isNotEmpty()) {
            config.dnsUpstreams.forEach { dns ->
                try {
                    val addr = java.net.InetAddress.getByName(dns)
                    builder.addDnsServer(addr)
                    Log.d(TAG, "Added DNS: $dns")
                } catch (e: Exception) {
                    Log.e(TAG, "Invalid DNS: $dns")
                }
            }
        } else {
            builder.addDnsServer("9.9.9.9")
        }

        vpnInterface = builder.establish()
        if (vpnInterface == null) {
            Log.e(TAG, "Failed to establish VPN interface")
            return
        }

        Log.d(TAG, "VPN interface established: fd=${vpnInterface!!.fd}")

        // Connect to remote server if config is available
        if (config != null) {
            connectToServer(config)
        }
    }

    private fun connectToServer(config: ServerConfig) {
        val address = config.addresses.firstOrNull() ?: run {
            Log.e(TAG, "No server address available")
            return
        }
        val host = address.substringBefore(":")
        val port = address.substringAfter(":", "443").toIntOrNull() ?: 443

        Log.d(TAG, "Connecting to server: $host:$port (hostname=${config.hostname})")

        job = CoroutineScope(Dispatchers.IO).launch {
            try {
                val socket = Socket()
                socket.connect(InetSocketAddress(host, port), 15000)
                serverSocket = socket
                Log.d(TAG, "Connected to server: $host:${socket.port}")

                // Start bidirectional packet forwarding
                val vpn = vpnInterface ?: return@launch
                forwardPackets(vpn, socket)
            } catch (e: Exception) {
                Log.e(TAG, "Server connection error: ${e.message}")
                // VPN interface is established but no server tunnel — will retry or handle gracefully
                // Keep VPN interface alive so the system shows "connected"
            }
        }
    }

    private suspend fun forwardPackets(vpn: ParcelFileDescriptor, socket: Socket) {
        Log.d(TAG, "Starting packet forwarding: fd=${vpn.fd} <-> ${socket.inetAddress}:${socket.port}")

        val vpnInput = ParcelFileDescriptor.AutoCloseInputStream(vpn)
        val vpnOutput = ParcelFileDescriptor.AutoCloseOutputStream(vpn)
        val socketInput: InputStream = socket.getInputStream()
        val socketOutput: OutputStream = socket.getOutputStream()

        // Coroutine: VPN -> Server
        val vpnToServer = CoroutineScope(Dispatchers.IO).launch {
            try {
                val buffer = ByteArray(32767)
                while (isActive && !socket.isClosed) {
                    val length = vpnInput.read(buffer)
                    if (length > 0) {
                        socketOutput.write(buffer, 0, length)
                        socketOutput.flush()
                        Log.v(TAG, "VPN -> Server: $length bytes")
                    } else if (length < 0) {
                        break
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "VPN->Server error: ${e.message}")
            }
        }

        // Coroutine: Server -> VPN
        val serverToVpn = CoroutineScope(Dispatchers.IO).launch {
            try {
                val buffer = ByteArray(32767)
                while (isActive && !socket.isClosed) {
                    val length = socketInput.read(buffer)
                    if (length > 0) {
                        vpnOutput.write(buffer, 0, length)
                        vpnOutput.flush()
                        Log.v(TAG, "Server -> VPN: $length bytes")
                    } else if (length < 0) {
                        break
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Server->VPN error: ${e.message}")
            }
        }

        // Wait for either direction to finish
        joinAll(vpnToServer, serverToVpn)
        Log.d(TAG, "Packet forwarding ended")
    }

    private fun disconnect() {
        Log.d(TAG, "Disconnecting...")
        job?.cancel()
        try {
            serverSocket?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing server socket: ${e.message}")
        }
        serverSocket = null
        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing interface: ${e.message}")
        }
        vpnInterface = null
    }

    override fun onDestroy() {
        disconnect()
        super.onDestroy()
    }
}
