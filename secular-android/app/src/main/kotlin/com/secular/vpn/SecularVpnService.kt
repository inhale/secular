// Secular VpnService — uses TrustTunnel native VpnClient for TLS/HTTP/2/QUIC tunneling.
//
// Architecture:
//   DashboardFragment → SecularVpnService.connect(serverJson)
//   SecularVpnService → VpnClient (com.adguard.trusttunnel) → JNI → libtrusttunnel_android.so
//   Native library handles: TLS, ALPN, HTTP/2 auth, IP packet relay

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
import com.adguard.trusttunnel.VpnClientListener
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.*
import java.io.File

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

        // Byte counters (used by DashboardFragment for display)
        // With native tunnel these are stubs — native lib manages its own counters
        @JvmField
        val bytesDownloaded = java.util.concurrent.atomic.AtomicLong(0)
        @JvmField
        val bytesUploaded = java.util.concurrent.atomic.AtomicLong(0)

        // TrustTunnel VpnState codes (match native enum)
        const val STATE_DISCONNECTED = 0
        const val STATE_CONNECTING = 1
        const val STATE_CONNECTED = 2
        const val STATE_WAITING_RECOVERY = 3
        const val STATE_RECOVERING = 4
        const val STATE_WAITING_FOR_NETWORK = 5

        // Service reference for JNI callbacks
        @Volatile
        var instance: SecularVpnService? = null

        // JNI callbacks (not used with AAR VpnClient, but kept for compatibility)
        @JvmStatic
        fun onNativeStateChanged(state: Int) {}
        @JvmStatic
        fun onNativeConnectionInfo(info: String) {}

        // Log buffer for LogFragment UI
        val logBuffer = mutableListOf<String>()
        fun addLog(msg: String) {
            val ts = java.text.SimpleDateFormat("HH:mm:ss.SSS", java.util.Locale.US)
                .format(java.util.Date())
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
        @SerializedName("upstream_protocol") val upstreamProtocol: String = "auto",
        @SerializedName("dns_upstreams") val dnsUpstreams: List<String> = emptyList(),
        @SerializedName("certificate") val certificate: String = "",
        @SerializedName("skip_verification") val skipVerification: Boolean = true,
        @SerializedName("anti_dpi") val antiDpi: Boolean = false,
        @SerializedName("client_random") val clientRandom: String = ""
    ) {
        fun toTrustTunnelToml(): String {
            val sb = StringBuilder()

            // Top-level settings
            sb.appendLine("vpn_mode = \"general\"")
            sb.appendLine("loglevel = \"debug\"")
            sb.appendLine("killswitch_enabled = false")
            sb.appendLine("post_quantum_group_enabled = false")
            sb.appendLine()

            // [listener.tun]
            sb.appendLine("[listener.tun]")
            sb.appendLine("included_routes = [\"0.0.0.0/0\", \"::/0\"]")
            sb.appendLine("excluded_routes = []")
            sb.appendLine("mtu_size = 1500")
            sb.appendLine("change_system_dns = false")
            sb.appendLine()

            // [endpoint] — server connection
            sb.appendLine("[endpoint]")

            val sni = hostname.ifEmpty {
                addresses.firstOrNull()?.substringBefore(":") ?: ""
            }

            val addrToml = if (addresses.isNotEmpty()) {
                addresses.joinToString(", ") { "\"$it\"" }
            } else {
                "\"0.0.0.0:443\""
            }

            sb.appendLine("hostname = \"$sni\"")
            sb.appendLine("addresses = [$addrToml]")
            sb.appendLine("username = \"$username\"")
            sb.appendLine("password = \"$password\"")

            val proto = when (upstreamProtocol) {
                "http3" -> "http3"
                else -> "auto"
            }
            sb.appendLine("upstream_protocol = \"$proto\"")

            val dnsList = if (dnsUpstreams.isNotEmpty()) {
                dnsUpstreams.joinToString(", ") { "\"$it\"" }
            } else {
                "\"9.9.9.9\", \"149.112.112.112\""
            }
            sb.appendLine("dns_upstreams = [$dnsList]")

            sb.appendLine("has_ipv6 = $hasIpv6")
            if (certificate.isNotEmpty()) {
                sb.appendLine("certificate = \"\"\"${certificate}\"\"\"")
            }
            sb.appendLine("skip_verification = $skipVerification")
            sb.appendLine("anti_dpi = $antiDpi")

            if (clientRandom.isNotEmpty()) {
                sb.appendLine("client_random = \"$clientRandom\"")
            }

            return sb.toString()
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        createNotificationChannel()
        addLog("Service created (TrustTunnel native)")

        // Catch anything that escapes our try/catch blocks (Error, not just Exception)
        Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
            Log.e(TAG, "FATAL UncaughtExceptionHandler: ${throwable.javaClass.name}: ${throwable.message}")
            // Write to file since log buffer might not survive
            try {
                val crashLog = File(getExternalFilesDir(null) ?: filesDir, "crash.log")
                crashLog.appendText("[${java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss").format(java.util.Date())}] FATAL on ${thread.name}: ${throwable.javaClass.name}: ${throwable.message}\n")
                throwable.stackTrace.forEach {
                    crashLog.appendText("  at $it\n")
                }
                // Also dump recent log buffer for context
                crashLog.appendText("\n--- Last log entries ---\n")
                synchronized(logBuffer) {
                    logBuffer.takeLast(50).forEach {
                        crashLog.appendText("  $it\n")
                    }
                }
            } catch (_: Exception) {}
            // Re-throw so we still get the native crash dump
            throwable.printStackTrace()
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        addLog("onStartCommand: action=$action, isTunnelUp=$isTunnelUp, isConnecting=$isConnecting")

        // Race condition guard: if we get DISCONNECT while still connecting,
        // ignore it — the connect coroutine will handle its own cleanup
        if (action == ACTION_DISCONNECT && isConnecting && !isTunnelUp) {
            addLog("onStartCommand: ignoring DISCONNECT while connecting")
            return START_NOT_STICKY
        }

        return when (action) {
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
                addLog("Unknown action: $action")
                START_NOT_STICKY
            }
        }
    }

    private fun connect(serverJson: String?) {
        addLog("connect() called, isTunnelUp=$isTunnelUp, isConnecting=$isConnecting")

        // If tunnel is already up, don't try to reconnect — just return
        if (isTunnelUp) {
            addLog("connect() — tunnel already up, ignoring")
            return
        }

        // If already connecting, don't start another attempt
        if (isConnecting) {
            addLog("connect() — already connecting, ignoring")
            return
        }

        val config = try {
            if (serverJson != null) {
                Gson().fromJson(serverJson, ServerConfig::class.java).also {
                    addLog("Config: name=${it.name} host=${it.hostname} addr=${it.addresses}")
                }
            } else {
                addLog("No server JSON")
                return
            }
        } catch (e: Exception) {
            addLog("Parse error: ${e.message}")
            lastError = "Invalid config: ${e.message}"
            return
        }

        if (config.addresses.isEmpty()) {
            lastError = "No server address"
            return
        }

        if (config.username.isEmpty() || config.password.isEmpty()) {
            lastError = "Server '${config.name}' has no username or password. Edit the server and add credentials."
            addLog("connectToServer: $lastError")
            isConnecting = false
            return
        }

        // Build VPN TUN interface
        addLog("Building VPN interface...")
        val builder = try {
            Builder()
                .setSession("Secular VPN")
                .setMtu(1500)
                .addAddress("172.20.2.13", 32)
                .addAddress("fdfd:29::2", 64)
                .addRoute("0.0.0.0", 0)
                .addDnsServer(java.net.InetAddress.getByName("9.9.9.9"))
                .setBlocking(true)
        } catch (e: Throwable) {
            addLog("Builder failed: ${e.javaClass.simpleName}: ${e.message}")
            lastError = "Failed to build VPN interface: ${e.message}"
            return
        }

        try {
            builder.addDisallowedApplication(packageName)
        } catch (_: Exception) {}

        vpnInterface = try { builder.establish() } catch (e: Throwable) {
            addLog("establish() failed: ${e.javaClass.simpleName}: ${e.message}")
            lastError = "VPN permission not granted"
            return
        }
        if (vpnInterface == null) {
            addLog("builder.establish() returned null — VPN not prepared?")
            lastError = "VPN permission not granted"
            return
        }

        addLog("VPN interface: fd=${vpnInterface!!.fd}")

        try {
            startForeground(NOTIFICATION_ID, buildNotification("Connecting..."))
        } catch (e: Exception) {
            addLog("Foreground failed: ${e.message}")
        }

        connectToServer(config)
    }

    private fun connectToServer(config: ServerConfig) {
        vpnJob = serviceScope.launch {
            try {
                isConnecting = true
                isTunnelUp = false
                lastError = null

                // Resolve certificate file path to PEM content
                val resolvedConfig = if (config.certificate.isNotEmpty() && !config.certificate.contains("BEGIN CERTIFICATE")) {
                    try {
                        val certFile = java.io.File(filesDir, config.certificate)
                        if (certFile.exists()) {
                            val certPem = certFile.readText()
                            config.copy(certificate = certPem)
                        } else config
                    } catch (_: Exception) { config }
                } else config

                val tomlConfig = resolvedConfig.toTrustTunnelToml()
                addLog("TOML config:")
                tomlConfig.lines().forEach { addLog("  $it") }
                // Also write TOML to file for post-crash inspection
                try {
                    val tomlFile = java.io.File(getExternalFilesDir(null) ?: filesDir, "last_config.toml")
                    tomlFile.writeText(tomlConfig)
                } catch (_: Exception) {}

                val vpn = vpnInterface
                if (vpn == null) {
                    lastError = "VPN interface lost"
                    isConnecting = false
                    return@launch
                }

                // Deferred to wait for async native connection result
                val connectDeferred = CompletableDeferred<Boolean>()

                // Create native client — AAR's constructor takes (tomlConfig, VpnClientListener)
                // and calls createNative() internally. If native lib fails to load, this throws.
                addLog("Creating native client...")
                val client = try {
                    VpnClient(tomlConfig, object : VpnClientListener {
                        override fun protectSocket(fd: Int): Boolean {
                            return try { protect(fd) } catch (_: Exception) { false }
                        }
                        override fun verifyCertificate(certificate: ByteArray?, rawChain: List<ByteArray?>?): Boolean {
                            return true // TODO: implement cert pinning
                        }
                        override fun onStateChanged(state: Int) {
                            addLog("Native state: $state")
                            updateNotificationForState(state)
                            when (state) {
                                STATE_CONNECTED -> {
                                    isTunnelUp = true
                                    isConnecting = false
                                    connectDeferred.complete(true)
                                }
                                STATE_DISCONNECTED -> {
                                    isTunnelUp = false
                                    isConnecting = false
                                    if (!connectDeferred.isCompleted) {
                                        connectDeferred.complete(false)
                                    }
                                }
                            }
                        }
                        override fun onConnectionInfo(info: String) {
                            // addLog("Native info: $info")  // verbose per-packet, disabled
                        }
                    })
                } catch (e: Throwable) {
                    addLog("VpnClient creation failed: ${e.javaClass.simpleName}: ${e.message}")
                    lastError = "Tunnel init failed: ${e.message}"
                    isConnecting = false
                    return@launch
                }

                nativeClient = client

                // Start the tunnel — AAR uses start(ParcelFileDescriptor), not start(int fd)
                addLog("Starting tunnel with fd=${vpn.fd}")
                val startResult = try {
                    client.start(vpn)
                } catch (e: Throwable) {
                    addLog("client.start() THREW: ${e.javaClass.simpleName}: ${e.message}")
                    false
                }
                addLog("client.start() returned: $startResult")

                if (!startResult) {
                    lastError = "Tunnel start failed"
                    isTunnelUp = false
                    isConnecting = false
                    client.stop()
                    client.close()
                    nativeClient = null
                    return@launch
                }

                // Wait for the native connection to complete (up to 15 seconds)
                addLog("Waiting for native connection...")
                val connected = try {
                    withTimeout(15000L) {
                        connectDeferred.await()
                    }
                } catch (e: TimeoutCancellationException) {
                    addLog("connectToServer: connection timed out after 15s")
                    false
                }

                if (connected) {
                    addLog("connectToServer: CONNECTED!")
                } else {
                    addLog("connectToServer: connection failed")
                    isTunnelUp = false
                    isConnecting = false
                    lastError = lastError ?: "Connection failed"
                    client.stop()
                    client.close()
                    nativeClient = null
                }

            } catch (e: CancellationException) {
                addLog("Connection cancelled")
                isTunnelUp = false
                isConnecting = false
            } catch (e: Throwable) {
                addLog("Connection failed: ${e.javaClass.simpleName}: ${e.message}")
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
            STATE_WAITING_RECOVERY, STATE_RECOVERING -> "Secular VPN recovering..."
            STATE_WAITING_FOR_NETWORK -> "Secular VPN waiting for network..."
            else -> "Secular VPN"
        }
        try { updateNotification(text) } catch (_: Exception) {}
    }

    private fun disconnect() {
        addLog("disconnect()")
        isTunnelUp = false
        isConnecting = false
        vpnJob?.cancel()
        vpnJob = null

        try { nativeClient?.stop() } catch (_: Exception) {}
        try { nativeClient?.close() } catch (_: Exception) {}
        nativeClient = null

        // Don't close vpnInterface — fd was already detached and given to native.
        // Closing the detached PFD is a no-op for the fd, but skip it anyway.
        vpnInterface = null

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
        addLog("Service destroyed, wasConnected=$isTunnelUp")
        instance = null
        disconnect()
        serviceScope.cancel()
        super.onDestroy()
    }
}
