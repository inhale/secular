// Android VpnService implementation using TrustTunnel native client
package com.secular.vpn

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.NotificationCompat
import com.adguard.trusttunnel.VpnClient
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.*

class SecularVpnService : VpnService() {
    companion object {
        const val TAG = "SecularVPN"
        const val ACTION_CONNECT = "com.secular.vpn.CONNECT"
        const val ACTION_DISCONNECT = "com.secular.vpn.DISCONNECT"
        const val CHANNEL_ID = "secular_vpn_channel"
        const val NOTIFICATION_ID = 1001

        @Volatile var isTunnelUp = false
            private set
        @Volatile var lastError: String? = null
            private set
        @Volatile var isConnecting = false
            private set
        val bytesDownloaded = java.util.concurrent.atomic.AtomicLong(0)
        val bytesUploaded = java.util.concurrent.atomic.AtomicLong(0)

        // Native state codes matching TrustTunnel VpnState enum
        const val STATE_DISCONNECTED = 0
        const val STATE_CONNECTING = 1
        const val STATE_CONNECTED = 2
        const val STATE_WAITING_RECOVERY = 3
        const val STATE_RECOVERING = 4
        const val STATE_WAITING_FOR_NETWORK = 5

        // Reference to the running service instance (for JNI callbacks)
        @Volatile
        var instance: SecularVpnService? = null

        // Called from JNI via NativeVpnClient
        @JvmStatic
        fun onNativeStateChanged(state: Int) {
            when (state) {
                STATE_CONNECTED -> {
                    isTunnelUp = true
                    isConnecting = false
                    lastError = null
                }
                STATE_DISCONNECTED -> {
                    isTunnelUp = false
                    isConnecting = false
                }
                STATE_CONNECTING -> {
                    isConnecting = true
                }
                else -> { /* recovery states */ }
            }
        }

        // Log buffer for UI
        val logBuffer = mutableListOf<String>()
        fun addLog(msg: String) {
            val ts = java.text.SimpleDateFormat("HH:mm:ss.SSS", java.util.Locale.US).format(java.util.Date())
            synchronized(logBuffer) {
                logBuffer.add("[$ts] $msg")
                if (logBuffer.size > 500) logBuffer.removeAt(0)
            }
            Log.d(TAG, msg)
        }
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var vpnJob: Job? = null
    private var nativeClient: VpnClient? = null
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private data class ServerConfig(
        @SerializedName("name") val name: String = "",
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
    ) {
        /**
         * Build a TrustTunnel client configuration TOML string.
         * The native client parses this with toml::parse().
         *
         * The config includes:
         * - [listener.tun] — TUN interface settings (routes, MTU)
         * - [endpoint] — server connection settings (address, protocol, DNS)
         */
        fun toTrustTunnelToml(): String {
            val host = addresses.firstOrNull()?.substringBefore(":") ?: ""
            val port = addresses.firstOrNull()?.substringAfter(":", "443") ?: "443"
            val sni = hostname.ifEmpty { host }

            val sb = StringBuilder()

            // [listener] section — TUN interface config
            sb.appendLine("[listener]")
            sb.appendLine("[listener.tun]")
            // Route all traffic through VPN
            sb.appendLine("included_routes = [\"0.0.0.0/0\", \"::/0\"]")
            sb.appendLine("excluded_routes = []")
            sb.appendLine("mtu_size = 1380")

            // [endpoint] section — server connection
            sb.appendLine()
            sb.appendLine("[endpoint]")
            sb.appendLine("address = \"$host:$port\"")
            sb.appendLine("sni = \"$sni\"")

            // Protocol
            when (upstreamProtocol) {
                "http3" -> sb.appendLine("protocol = \"http3\"")
                "http2" -> sb.appendLine("protocol = \"http2\"")
                "http1" -> sb.appendLine("protocol = \"http1\"")
                else -> sb.appendLine("protocol = \"http2\"")
            }

            // DNS upstreams
            val dnsList = if (dnsUpstreams.isNotEmpty()) {
                dnsUpstreams
            } else {
                listOf("9.9.9.9", "149.112.112.112")  // Quad9 defaults
            }
            sb.appendLine("dns_upstreams = [${dnsList.joinToString(", ") { "\"$it\"" }}]")

            // Authentication
            if (username.isNotEmpty()) {
                sb.appendLine("username = \"$username\"")
            }
            if (password.isNotEmpty()) {
                sb.appendLine("password = \"$password\"")
            }

            // Anti-DPI
            sb.appendLine("anti_dpi = $antiDpi")

            // Client random
            if (clientRandom.isNotEmpty()) {
                sb.appendLine("client_random = \"$clientRandom\"")
            }

            // Skip certificate verification
            sb.appendLine("skip_verification = $skipVerification")

            return sb.toString()
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        createNotificationChannel()
        addLog("Service created (native TrustTunnel)")
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        addLog("onStartCommand: action=${intent?.action}")
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
            else -> {
                addLog("Unknown action: ${intent?.action}")
                START_NOT_STICKY
            }
        }
    }

    private fun connect(serverJson: String?) {
        addLog("connect() called, serverJson=${serverJson != null}")

        val config = try {
            if (serverJson != null) {
                Gson().fromJson(serverJson, ServerConfig::class.java).also {
                    addLog("Parsed config: name=${it.name} host=${it.hostname} addr=${it.addresses}")
                }
            } else {
                addLog("No server JSON provided")
                null
            }
        } catch (e: Exception) {
            addLog("Parse error: ${e.message}")
            lastError = "Invalid config: ${e.message}"
            return
        }

        if (config == null) {
            lastError = "No server config"
            return
        }

        if (config.addresses.isEmpty()) {
            addLog("No address in config")
            lastError = "No server address"
            return
        }

        // Build VPN TUN interface
        addLog("Establishing VPN interface...")
        val builder = Builder()
            .setSession("Secular VPN")
            .setMtu(1380)
            .addAddress("10.0.0.2", 32)
            .addRoute("0.0.0.0", 0)
            .addDnsServer(java.net.InetAddress.getByName("9.9.9.9"))
            .setBlocking(true)

        if (config.dnsUpstreams.isNotEmpty()) {
            config.dnsUpstreams.forEach { dns ->
                try {
                    builder.addDnsServer(java.net.InetAddress.getByName(dns))
                    addLog("Added DNS: $dns")
                } catch (e: Exception) { addLog("Bad DNS: $dns") }
            }
        }

        vpnInterface = builder.establish()
        if (vpnInterface == null) {
            addLog("builder.establish() returned null — VPN not prepared?")
            lastError = "VPN permission not granted. Accept the system VPN dialog first."
            return
        }

        addLog("VPN interface established: fd=${vpnInterface!!.fd}")

        try {
            startForeground(NOTIFICATION_ID, buildNotification("Connecting..."))
            addLog("Foreground notification started")
        } catch (e: Exception) {
            addLog("Foreground start failed: ${e.message}")
        }

        connectToServer(config)
    }

    private fun connectToServer(config: ServerConfig) {
        val host = config.addresses.firstOrNull()?.substringBefore(":") ?: ""
        val port = config.addresses.firstOrNull()?.substringAfter(":", "443")?.toIntOrNull() ?: 443
        addLog("Connecting to $host:$port via native TrustTunnel client")

        vpnJob = serviceScope.launch {
            try {
                isConnecting = true
                isTunnelUp = false
                lastError = null
                bytesDownloaded.set(0)
                bytesUploaded.set(0)

                val tomlConfig = config.toTrustTunnelToml()
                addLog("TOML config:")
                tomlConfig.lines().forEach { addLog("  $it") }

                val client = VpnClient(
                    config = tomlConfig,
                    listener = object : VpnClient.Listener {
                        override fun onStateChanged(state: Int) {
                            addLog("Native state: $state")
                            onNativeStateChanged(state)
                            updateNotificationForState(state)
                        }

                        override fun onConnectionInfo(info: String) {
                            addLog("Native info: $info")
                        }
                    }
                )

                nativeClient = client

                val vpn = vpnInterface
                if (vpn == null) {
                    addLog("VPN interface null, cancelling")
                    lastError = "VPN interface lost"
                    isTunnelUp = false
                    isConnecting = false
                    return@launch
                }

                addLog("Starting native tunnel with tunFd=${vpn.fd}")
                // This is synchronous — native client runs the full tunnel lifecycle
                val result = client.start(vpn.fd)
                addLog("Native tunnel returned: result=$result")

                isTunnelUp = false
                isConnecting = false

                if (!result && lastError == null) {
                    lastError = "Tunnel disconnected unexpectedly"
                }

            } catch (e: CancellationException) {
                addLog("Connection cancelled")
            } catch (e: Exception) {
                addLog("Native connection failed: ${e.javaClass.simpleName}: ${e.message}")
                lastError = "Connection failed: ${e.message}"
                isTunnelUp = false
                isConnecting = false
            }
        }
    }

    private fun updateNotificationForState(state: Int) {
        val text = when (state) {
            STATE_CONNECTED -> "Secular VPN connected"
            STATE_CONNECTING -> "Secular VPN connecting..."
            STATE_DISCONNECTED -> "Secular VPN disconnected"
            STATE_WAITING_RECOVERY -> "Secular VPN recovering..."
            STATE_RECOVERING -> "Secular VPN recovering..."
            STATE_WAITING_FOR_NETWORK -> "Secular VPN waiting for network..."
            else -> "Secular VPN"
        }
        try { updateNotification(text) } catch (_: Exception) {}
    }

    private fun disconnect() {
        addLog("disconnect() called")
        isTunnelUp = false
        isConnecting = false
        vpnJob?.cancel()
        vpnJob = null

        try {
            nativeClient?.stop()
        } catch (e: Exception) {
            addLog("nativeClient.stop() error: ${e.message}")
        }
        try {
            nativeClient?.destroy()
        } catch (e: Exception) {
            addLog("nativeClient.destroy() error: ${e.message}")
        }
        nativeClient = null

        try { vpnInterface?.close() } catch (_: Exception) {}
        vpnInterface = null

        bytesDownloaded.set(0)
        bytesUploaded.set(0)

        try { stopForeground(STOP_FOREGROUND_REMOVE) } catch (_: Exception) {}
        stopSelf()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val nm = getSystemService(NotificationManager::class.java)
            nm?.createNotificationChannel(
                NotificationChannel(CHANNEL_ID, "VPN", NotificationManager.IMPORTANCE_LOW)
            )
        }
    }

    private fun buildNotification(text: String): Notification {
        val intent = Intent(this, com.secular.vpn.MainActivity::class.java)
        val pi = PendingIntent.getActivity(this, 0, intent, PendingIntent.FLAG_IMMUTABLE)
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Secular VPN")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentIntent(pi)
            .setOngoing(true)
            .build()
    }

    private fun updateNotification(text: String) {
        val nm = getSystemService(NotificationManager::class.java)
        nm?.notify(NOTIFICATION_ID, buildNotification(text))
    }

    override fun onDestroy() {
        addLog("Service destroyed")
        instance = null
        disconnect()
        serviceScope.cancel()
        super.onDestroy()
    }
}
