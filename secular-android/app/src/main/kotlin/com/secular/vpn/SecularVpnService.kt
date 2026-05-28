// Android VpnService implementation
package com.secular.vpn

import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.ParcelFileDescriptor
import android.util.Log
import kotlinx.coroutines.*

class SecularVpnService : VpnService() {
    companion object {
        const val TAG = "SecularVPN"
        const val ACTION_CONNECT = "com.secular.vpn.CONNECT"
        const val ACTION_DISCONNECT = "com.secular.vpn.DISCONNECT"
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var job: Job? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        return when (intent?.action) {
            ACTION_CONNECT -> {
                connect()
                START_STICKY
            }
            ACTION_DISCONNECT -> {
                disconnect()
                START_NOT_STICKY
            }
            else -> START_NOT_STICKY
        }
    }

    private fun connect() {
        Log.d(TAG, "Connecting...")

        val builder = Builder()
            .setSession("Secular")
            .setMtu(1380)
            .addAddress("10.0.0.2", 32)
            .addRoute("0.0.0.0", 0)  // Route all traffic
            .addDnsServer("9.9.9.9")
            .setBlocking(true)

        // Allow bypass for local network (optional)
        // builder.addDisallowedApplication("com.some.app")

        vpnInterface = builder.establish()
        if (vpnInterface == null) {
            Log.e(TAG, "Failed to establish VPN interface")
            return
        }

        Log.d(TAG, "VPN interface established: ${vpnInterface!!.fd}")

        // Start packet forwarding coroutine
        job = CoroutineScope(Dispatchers.IO).launch {
            try {
                forwardPackets(vpnInterface!!.fd)
            } catch (e: Exception) {
                Log.e(TAG, "Packet forwarding error: ${e.message}")
            }
        }
    }

    private fun disconnect() {
        Log.d(TAG, "Disconnecting...")
        job?.cancel()
        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing interface: ${e.message}")
        }
        vpnInterface = null
    }

    /**
     * Forward packets between the TUN interface and the Rust core.
     * 
     * Reads raw IP packets from the TUN fd, passes them to secular-core
     * via JNI/UniFFI, and writes response packets back to the TUN.
     */
    private suspend fun forwardPackets(tunFd: Int) {
        Log.d(TAG, "Starting packet forwarding on fd=$tunFd")
        
        // TODO: Initialize secular-core via UniFFI
        // val engine = SecularCore.create(config)
        // engine.connect()
        
        val buffer = ByteArray(32767)
        
        withContext(Dispatchers.IO) {
            val inputStream = FileInputStream(tunFd)
            val outputStream = FileOutputStream(tunFd)
            
            while (isActive) {
                try {
                    val length = inputStream.read(buffer)
                    if (length > 0) {
                        // TODO: Pass to secular-core for encryption/wrapping
                        // val encrypted = engine.processPacket(buffer, length)
                        // outputStream.write(encrypted)
                    }
                } catch (e: Exception) {
                    Log.e(TAG, "Packet read error: ${e.message}")
                    break
                }
            }
        }
    }

    override fun onDestroy() {
        disconnect()
        super.onDestroy()
    }
}
