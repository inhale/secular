package com.adguard.trusttunnel

import com.secular.vpn.SecularVpnService
import java.io.Closeable

/**
 * JNI bridge to libtrusttunnel_android.so
 *
 * MUST be in com.adguard.trusttunnel package — the native library's JNI
 * methods are registered with these exact class names:
 *   Java_com_adguard_trusttunnel_VpnClient_createNative
 *   Java_com_adguard_trusttunnel_VpnClient_startNative
 *   Java_com_adguard_trusttunnel_VpnClient_stopNative
 *   Java_com_adguard_trusttunnel_VpnClient_destroyNative
 *
 * The native code calls back to these methods on the VpnClient instance:
 *   protectSocket(I)Z
 *   verifyCertificate([BLjava/util/List;)Z
 *   onStateChanged(I)V
 *   onConnectionInfo(Ljava/lang/String;)V
 */
class VpnClient(
    private val config: String,
    private val listener: Listener? = null
) : Closeable {

    interface Listener {
        fun onStateChanged(state: Int)
        fun onConnectionInfo(info: String)
    }

    companion object {
        @Volatile var nativeLibLoaded = false
            private set

        init {
            try {
                System.loadLibrary("trusttunnel_android")
                nativeLibLoaded = true
                // addLog may not be safe here (service not created yet), use Log directly
                android.util.Log.d("SecularVPN", "VpnClient: native library loaded")
            } catch (e: Throwable) {
                nativeLibLoaded = false
                android.util.Log.e("SecularVPN", "VpnClient: FAILED to load native lib: ${e.message}")
                // Don't re-throw — let the app show a nice error instead of crashing
            }
        }

        @JvmStatic
        private external fun excludeCidr(
            includedRoutes: Array<String>,
            excludedRoutes: Array<String>
        ): Array<String>?

        @JvmStatic
        private external fun setSystemDnsServersNative(
            servers: Array<String>,
            bootstraps: Array<String>?
        ): Boolean
    }

    private var nativePtr: Long = 0

    // Native methods from libtrusttunnel_android.so
    private external fun createNative(config: String): Long
    private external fun startNative(nativePtr: Long, tunFd: Int): Boolean
    private external fun stopNative(nativePtr: Long)
    private external fun destroyNative(nativePtr: Long)
    private external fun notifyNetworkChangeNative(nativePtr: Long, available: Boolean)

    // Called from native code via JNI — protect tunnel socket from VPN
    @Suppress("unused")
    fun protectSocket(socket: Int): Boolean {
        return try {
            val svc = SecularVpnService.instance
            if (svc != null) {
                svc.protect(socket)
            } else {
                false
            }
        } catch (e: Throwable) {
            false
        }
    }

    // Called from native code via JNI — verify certificate chain
    @Suppress("unused")
    fun verifyCertificate(certificate: ByteArray?, rawChain: List<ByteArray?>?): Boolean {
        // Accept all certs for now — CertificateVerificator can be added later
        return true
    }

    // Called from native code via JNI — VPN state changes
    @Suppress("unused")
    fun onStateChanged(state: Int) {
        try {
            SecularVpnService.onNativeStateChanged(state)
            listener?.onStateChanged(state)
        } catch (e: Throwable) {
            // Don't let listener exceptions crash the native thread
        }
    }

    // Called from native code via JNI — connection events
    @Suppress("unused")
    fun onConnectionInfo(info: String) {
        try {
            SecularVpnService.onNativeConnectionInfo(info)
            listener?.onConnectionInfo(info)
        } catch (e: Throwable) {
            // Don't let listener exceptions crash the native thread
        }
    }

    // Create native client from TOML config string
    fun create(): Boolean {
        SecularVpnService.addLog("VpnClient: create() — calling createNative, config length=${config.length}")
        SecularVpnService.addLog("VpnClient: create() — config preview: ${config.take(200)}")
        try {
            nativePtr = createNative(config)
            // BUG WORKAROUND: createNative returns 1 (not 0) on TOML parse failure.
            // Any value that's not a valid heap pointer is treated as failure.
            // Valid pointers from make_unique are at least a few KB into heap.
            if (nativePtr == 0L || nativePtr == 1L) {
                SecularVpnService.addLog("VpnClient: createNative FAILED (returned $nativePtr)")
                nativePtr = 0
                return false
            }
            SecularVpnService.addLog("VpnClient: created ptr=$nativePtr")
            return true
        } catch (e: Throwable) {
            SecularVpnService.addLog("VpnClient: createNative THREW: ${e.javaClass.simpleName}: ${e.message}")
            nativePtr = 0
            return false
        }
    }

    // Start tunnel with TUN file descriptor
    fun start(tunFd: Int): Boolean {
        SecularVpnService.addLog("VpnClient: start(fd=$tunFd) — calling startNative, ptr=$nativePtr")
        if (nativePtr == 0L) {
            SecularVpnService.addLog("VpnClient: start() called with null nativePtr")
            return false
        }
        try {
            val result = startNative(nativePtr, tunFd)
            SecularVpnService.addLog("VpnClient: startNative returned $result")
            return result
        } catch (e: Throwable) {
            SecularVpnService.addLog("VpnClient: startNative THREW: ${e.javaClass.simpleName}: ${e.message}")
            return false
        }
    }

    // Stop tunnel
    fun stop() {
        if (nativePtr != 0L && nativeLibLoaded) {
            try { stopNative(nativePtr) } catch (_: Throwable) {}
        }
    }

    // Destroy native client
    fun destroy() {
        if (nativePtr != 0L && nativeLibLoaded) {
            try { destroyNative(nativePtr) } catch (_: Throwable) {}
            nativePtr = 0
        }
    }

    override fun close() {
        destroy()
    }
}
