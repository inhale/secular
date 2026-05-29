// secular-android/app/src/main/kotlin/com/secular/vpn/MainActivity.kt
// Secular Android — Main Activity

package com.secular.vpn

import android.app.Activity
import android.content.Intent
import android.net.VpnService
import android.os.Bundle
import android.util.Log
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch

class MainActivity : AppCompatActivity() {

    companion object {
        const val TAG = "SecularMain"
        const val VPN_REQUEST_CODE = 1001
    }

    private var isConnected = false

    private val vpnPrepareLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            startVpnService()
        } else {
            Log.w(TAG, "VPN permission denied by user")
            Toast.makeText(this, "VPN permission required", Toast.LENGTH_SHORT).show()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val connectBtn = findViewById<Button>(R.id.connect_button)
        val statusText = findViewById<TextView>(R.id.status_text)

        connectBtn.setOnClickListener {
            if (isConnected) {
                stopVpnService()
                isConnected = false
                statusText.text = "Disconnected"
                connectBtn.text = "Connect"
            } else {
                prepareAndConnect()
            }
        }
    }

    private fun prepareAndConnect() {
        val intent = VpnService.prepare(this)
        if (intent != null) {
            // System dialog will ask for permission
            vpnPrepareLauncher.launch(intent)
        } else {
            // Already permitted
            startVpnService()
        }
    }

    private fun startVpnService() {
        val intent = Intent(this, SecularVpnService::class.java).apply {
            action = SecularVpnService.ACTION_CONNECT
        }
        startService(intent)
        isConnected = true
        findViewById<TextView>(R.id.status_text).text = "Connected"
        findViewById<Button>(R.id.connect_button).text = "Disconnect"
        Log.d(TAG, "VPN service started")
    }

    private fun stopVpnService() {
        val intent = Intent(this, SecularVpnService::class.java).apply {
            action = SecularVpnService.ACTION_DISCONNECT
        }
        startService(intent)
        Log.d(TAG, "VPN service stopped")
    }
}
