// Android VpnService implementation
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
        const val CHANNEL_ID = "secular_vpn_channel"
        const val NOTIFICATION_ID = 1001

        @Volatile var isTunnelUp = false
            private set
        @Volatile var lastError: String? = null
            private set
        @Volatile var isConnecting = false
            private set
        val bytesDownloaded = AtomicLong(0)
        val bytesUploaded = AtomicLong(0)

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
    private var serverSocket: Socket? = null
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
    )

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        addLog("Service created")
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

        // Parse server config
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

        val address = config.addresses.firstOrNull()
        if (address.isNullOrEmpty()) {
            addLog("No address in config")
            lastError = "No server address"
            return
        }

        // Build VPN interface
        addLog("Establishing VPN interface...")
        val builder = Builder()
            .setSession("Secular VPN")
            .setMtu(1380)
            .addAddress("10.0.0.2", 32)
            .addRoute("0.0.0.0", 0)
            .setBlocking(true)

        if (config.dnsUpstreams.isNotEmpty()) {
            config.dnsUpstreams.forEach { dns ->
                try {
                    builder.addDnsServer(java.net.InetAddress.getByName(dns))
                    addLog("Added DNS: $dns")
                } catch (e: Exception) { addLog("Bad DNS: $dns") }
            }
        } else {
            builder.addDnsServer(java.net.InetAddress.getByName("9.9.9.9"))
        }

        vpnInterface = builder.establish()
        if (vpnInterface == null) {
            addLog("builder.establish() returned null — VPN not prepared?")
            lastError = "VPN permission not granted. Accept the system VPN dialog first."
            return
        }

        addLog("VPN interface established: fd=${vpnInterface!!.fd}")

        // Start as foreground service so Android doesn't kill us
        try {
            startForeground(NOTIFICATION_ID, buildNotification("Connecting..."))
            addLog("Foreground notification started")
        } catch (e: Exception) {
            addLog("Foreground start failed: ${e.message}")
        }

        // Connect to remote server
        connectToServer(config, address)
    }

    private fun connectToServer(config: ServerConfig, address: String) {
        val host = address.substringBefore(":")
        val port = address.substringAfter(":", "443").toIntOrNull() ?: 443

        addLog("Connecting TCP to $host:$port")

        vpnJob = serviceScope.launch {
            try {
                isConnecting = true
                isTunnelUp = false
                lastError = null
                bytesDownloaded.set(0)
                bytesUploaded.set(0)

                val socket = Socket()
                addLog("Opening socket to $host:$port with 15s timeout...")
                withContext(Dispatchers.IO) {
                    socket.connect(InetSocketAddress(host, port), 15000)
                }
                socket.tcpNoDelay = true
                serverSocket = socket
                addLog("TCP connected: remote=${socket.inetAddress}:${socket.port}")
                isTunnelUp = true
                isConnecting = false

                updateNotification("Connected to $host")

                val vpn = vpnInterface
                if (vpn == null) {
                    addLog("VPN interface null, cancelling")
                    lastError = "VPN interface lost"
                    isTunnelUp = false
                    return@launch
                }

                addLog("Starting packet forwarding: fd=${vpn.fd}")
                forwardPackets(vpn, socket)
            } catch (e: CancellationException) {
                addLog("Connection cancelled")
                // normal on disconnect
            } catch (e: Exception) {
                addLog("Server connection failed: ${e.javaClass.simpleName}: ${e.message}")
                lastError = "Connection failed: ${e.message}"
                isTunnelUp = false
                isConnecting = false
            }
        }
    }

    private suspend fun forwardPackets(vpn: ParcelFileDescriptor, socket: Socket) {
        val vpnInput = ParcelFileDescriptor.AutoCloseInputStream(vpn)
        val vpnOutput = ParcelFileDescriptor.AutoCloseOutputStream(vpn)
        val sockInput: InputStream = socket.getInputStream()
        val sockOutput: OutputStream = socket.getOutputStream()
        addLog("Packet forwarding started")

        val vpnToServer = serviceScope.launch(Dispatchers.IO) {
            try {
                val buf = ByteArray(32767)
                while (isActive && !socket.isClosed) {
                    val len = vpnInput.read(buf)
                    if (len > 0) {
                        sockOutput.write(buf, 0, len)
                        sockOutput.flush()
                        bytesUploaded.addAndGet(len.toLong())
                    } else if (len < 0) break
                }
            } catch (_: Exception) {}
        }

        val serverToVpn = serviceScope.launch(Dispatchers.IO) {
            try {
                val buf = ByteArray(32767)
                while (isActive && !socket.isClosed) {
                    val len = sockInput.read(buf)
                    if (len > 0) {
                        vpnOutput.write(buf, 0, len)
                        vpnOutput.flush()
                        bytesDownloaded.addAndGet(len.toLong())
                    } else if (len < 0) break
                }
            } catch (_: Exception) {}
        }

        joinAll(vpnToServer, serverToVpn)
        addLog("Packet forwarding ended")
        isTunnelUp = false
    }

    private fun disconnect() {
        addLog("disconnect() called")
        isTunnelUp = false
        isConnecting = false
        vpnJob?.cancel()
        vpnJob = null

        try { serverSocket?.close() } catch (_: Exception) {}
        serverSocket = null

        try { vpnInterface?.close() } catch (_: Exception) {}
        vpnInterface = null

        bytesDownloaded.set(0)
        bytesUploaded.set(0)

        try { stopForeground(STOP_FOREGROUND_REMOVE) } catch (_: Exception) {}
        stopSelf()
    }

    // ── Notifications ──
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
            .setSmallIcon(android.R.drawable.ic_lock_lock) // use any available icon; TODO: proper icon
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
        disconnect()
        serviceScope.cancel()
        super.onDestroy()
    }
}
