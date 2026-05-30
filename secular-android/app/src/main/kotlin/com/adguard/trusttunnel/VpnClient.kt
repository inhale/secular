// secular-android/app/src/main/kotlin/com/secular/vpn/NativeVpnClient.kt
// Thin JNI bridge matching the native library's expected class name.
// The native libtrusttunnel_android.so exports symbols like:
//   Java_com_adguard_trusttunnel_VpnClient_createNative
//   Java_com_adguard_trusttunnel_VpnClient_startNative
//   Java_com_adguard_trusttunnel_VpnClient_stopNative
//   Java_com_adguard_trusttunnel_VpnClient_destroyNative
// We create this class in the com.adguard.trusttunnel package so JNI finds them.

package com.adguard.trusttunnel

class VpnClient(
    private val config: String,
    private val listener: Listener? = null
) {
    private var nativePtr: Long = 0

    interface Listener {
        fun onStateChanged(state: Int)
        fun onConnectionInfo(info: String)
    }

    companion object {
        init {
            System.loadLibrary("trusttunnel_android")
        }
    }

    fun start(tunFd: Int): Boolean {
        nativePtr = createNative(config)
        if (nativePtr == 0L) return false
        return startNative(nativePtr, tunFd)
    }

    fun stop() {
        if (nativePtr != 0L) {
            stopNative(nativePtr)
        }
    }

    fun destroy() {
        if (nativePtr != 0L) {
            destroyNative(nativePtr)
            nativePtr = 0
        }
    }

    /**
     * Called by native code via JNI — protect this socket from VPN routing
     */
    @Suppress("unused")
    private fun protectSocket(socket: Int): Boolean {
        return try {
            SecularVpnService.instance?.protect(socket) ?: false
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Called by native code via JNI — verify server certificate chain.
     * For now accepts all certs (TODO: implement pinning).
     */
    @Suppress("unused")
    private fun verifyCertificate(certificate: ByteArray?, rawChain: List<ByteArray?>?): Boolean {
        return true
    }

    /**
     * Called by native code via JNI — VPN state change notification
     */
    @Suppress("unused")
    private fun onStateChanged(state: Int) {
        listener?.onStateChanged(state)
    }

    /**
     * Called by native code via JNI — connection info event
     */
    @Suppress("unused")
    private fun onConnectionInfo(info: String) {
        listener?.onConnectionInfo(info)
    }

    // Native JNI methods — symbols in libtrusttunnel_android.so
    private external fun createNative(config: String): Long
    private external fun startNative(nativePtr: Long, tunFd: Int): Boolean
    private external fun stopNative(nativePtr: Long)
    private external fun destroyNative(nativePtr: Long)
}
