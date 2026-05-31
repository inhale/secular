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
        init {
            try {
                System.loadLibrary("trusttunnel_android")
                SecularVpnService.addLog("VpnClient: native library loaded")
            } catch (e: UnsatisfiedLinkError) {
                SecularVpnService.addLog("VpnClient: FAILED to load libtrusttunnel_android.so: ${e.message}")
                throw e
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
        SecularVpnService.addLog("VpnClient: protectSocket($socket)")
        val svc = SecularVpnService.instance
        return if (svc != null) {
            svc.protectSocket(socket)
        } else {
            SecularVpnService.addLog("VpnClient: no service instance")
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
        SecularVpnService.addLog("VpnClient: state=$state")
        SecularVpnService.onNativeStateChanged(state)
        listener?.onStateChanged(state)
    }

    // Called from native code via JNI — connection events
    @Suppress("unused")
    fun onConnectionInfo(info: String) {
        SecularVpnService.addLog("VpnClient: $info")
        SecularVpnService.onNativeConnectionInfo(info)
        listener?.onConnectionInfo(info)
    }

    // Create native client from TOML config string
    fun create(): Boolean {
        SecularVpnService.addLog("VpnClient: create()")
        nativePtr = createNative(config)
        if (nativePtr == 0L) {
            SecularVpnService.addLog("VpnClient: createNative FAILED")
            return false
        }
        SecularVpnService.addLog("VpnClient: created ptr=$nativePtr")
        return true
    }

    // Start tunnel with TUN file descriptor
    fun start(tunFd: Int): Boolean {
        SecularVpnService.addLog("VpnClient: start(fd=$tunFd)")
        if (nativePtr == 0L) {
            SecularVpnService.addLog("VpnClient: not created yet")
            return false
        }
        return startNative(nativePtr, tunFd)
    }

    // Stop tunnel
    fun stop() {
        if (nativePtr != 0L) {
            SecularVpnService.addLog("VpnClient: stop()")
            stopNative(nativePtr)
        }
    }

    // Destroy native client
    fun destroy() {
        if (nativePtr != 0L) {
            SecularVpnService.addLog("VpnClient: destroy()")
            destroyNative(nativePtr)
            nativePtr = 0
        }
    }

    override fun close() {
        destroy()
    }
}
