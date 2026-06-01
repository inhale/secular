// secular-android/app/src/main/java/com/secular/vpn/ui/DashboardFragment.kt
// Dashboard screen — main VPN connection screen

package com.secular.vpn.ui

import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.VpnService
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.widget.*
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.navigation.fragment.findNavController
import com.secular.vpn.R
import com.secular.vpn.SecularVpnService
import com.secular.vpn.data.ServersRepository
import kotlinx.coroutines.launch

class DashboardFragment : Fragment() {

    private lateinit var repository: ServersRepository
    private lateinit var prefs: SharedPreferences
    private var isConnected = false
    private var seconds = 0L
    private val handler = Handler(Looper.getMainLooper())
    private var timerRunnable: Runnable? = null
    private var metricsRunnable: Runnable? = null
    private var selectedServerName: String? = null
    private var ringAnimator: android.animation.ObjectAnimator? = null

    private lateinit var vpnPrepareLauncher: ActivityResultLauncher<Intent>

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        vpnPrepareLauncher = registerForActivityResult(
            ActivityResultContracts.StartActivityForResult()
        ) { result ->
            if (result.resultCode == android.app.Activity.RESULT_OK) {
                SecularVpnService.addLog("VPN prepare OK — starting service")
                startVpnService()
            } else {
                SecularVpnService.addLog("VPN prepare DENIED by user")
                isConnected = false
                view?.let { updateUiDisconnected(it) }
            }
        }
    }

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_dashboard, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)
        repository = ServersRepository(requireContext())
        prefs = requireContext().getSharedPreferences("secular_vpn_prefs", Context.MODE_PRIVATE)

        view.findViewById<FrameLayout>(R.id.connect_btn).setOnClickListener {
            if (isConnected) disconnectVpn() else connectVpn()
        }

        view.findViewById<LinearLayout>(R.id.server_card).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_serverList)
        }

        view.findViewById<ImageButton>(R.id.btn_log).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_log)
        }
        view.findViewById<ImageButton>(R.id.nav_log).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_log)
        }
        view.findViewById<FrameLayout>(R.id.nav_home_btn).setOnClickListener { /* already home */ }
        view.findViewById<ImageButton>(R.id.nav_add).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_addServer)
        }

        loadSelectedServer(view)
    }

    private fun loadSelectedServer(view: View) {
        lifecycleScope.launch {
            try {
                val servers = repository.loadServers()
                val nameTv = view.findViewById<TextView>(R.id.server_card_name)
                val metaTv = view.findViewById<TextView>(R.id.server_card_meta)

                if (servers.isNotEmpty()) {
                    val savedName = prefs.getString("selected_server_name", null)
                    selectedServerName = savedName
                    val server = if (savedName != null) {
                        servers.find { it.name == savedName } ?: servers[0]
                    } else {
                        servers[0]
                    }
                    nameTv.text = server.name
                    metaTv.text = "TrustTunnel · ${server.displayAddress}"
                } else {
                    selectedServerName = null
                    nameTv.text = "No server selected"
                    metaTv.text = "Tap to add a server"
                }
            } catch (_: Exception) {}
        }
    }

    private fun connectVpn() {
        lifecycleScope.launch {
            try {
                val servers = repository.loadServers()
                if (servers.isEmpty()) {
                    Toast.makeText(requireContext(), "Add a server first", Toast.LENGTH_SHORT).show()
                    return@launch
                }
                val savedName = prefs.getString("selected_server_name", null)
                val server = if (savedName != null) {
                    servers.find { it.name == savedName } ?: servers[0]
                } else {
                    servers[0]
                }
                SecularVpnService.addLog("Connect tapped: server=${server.name}")

                val prepareIntent = try {
                    VpnService.prepare(requireContext())
                } catch (e: Throwable) {
                    SecularVpnService.addLog("VpnService.prepare() failed: ${e.message}")
                    Toast.makeText(requireContext(), "VPN not available: ${e.message}", Toast.LENGTH_SHORT).show()
                    return@launch
                }

                if (prepareIntent != null) {
                    isConnected = true
                    updateUiConnecting()
                    vpnPrepareLauncher.launch(prepareIntent)
                } else {
                    isConnected = true
                    updateUiConnecting()
                    startVpnService()
                }
            } catch (e: Throwable) {
                SecularVpnService.addLog("connectVpn error: ${e.javaClass.simpleName}: ${e.message}")
                isConnected = false
                view?.let { updateUiDisconnected(it) }
                try { Toast.makeText(requireContext(), "Connection error: ${e.message}", Toast.LENGTH_LONG).show() } catch (_: Exception) {}
            }
        }
    }

    private fun startVpnService() {
        lifecycleScope.launch {
            try {
                val servers = repository.loadServers()
                if (servers.isEmpty()) return@launch
                val savedName = prefs.getString("selected_server_name", null)
                val server = if (savedName != null) {
                    servers.find { it.name == savedName } ?: servers[0]
                } else {
                    servers[0]
                }
                val json = com.google.gson.Gson().toJson(server)
                val intent = Intent(requireContext(), SecularVpnService::class.java).apply {
                    action = SecularVpnService.ACTION_CONNECT
                    putExtra("server_json", json)
                }
                requireContext().startService(intent)
                startStatePolling()
            } catch (e: Throwable) {
                SecularVpnService.addLog("startVpnService error: ${e.javaClass.simpleName}: ${e.message}")
            }
        }
    }

    private fun startStatePolling() {
        handler.postDelayed(object : Runnable {
            override fun run() {
                if (!isConnected || view == null) return
                if (SecularVpnService.isTunnelUp) {
                    val v = view ?: return
                    updateUiConnected(v)
                    startMetricsPolling()
                } else if (SecularVpnService.lastError != null) {
                    val v = view ?: return
                    val err = SecularVpnService.lastError ?: "Connection failed"
                    v.findViewById<TextView>(R.id.status_label)?.text = err
                    v.findViewById<TextView>(R.id.status_label)?.setTextColor(resources.getColor(R.color.red, null))
                    disconnectVpn()
                } else {
                    handler.postDelayed(this, 1000)
                }
            }
        }, 2000)
    }

    private var lastTunDl: Long = 0
    private var lastTunUl: Long = 0

    private fun readTunBytes(): Pair<Long, Long> {
        return try {
            val dev = java.io.File("/proc/net/dev").readText()
            for (line in dev.lines()) {
                if (line.trim().startsWith("tun0:")) {
                    val parts = line.trim().split("\\s+".toRegex())
                    val rx = parts[1].toLongOrNull() ?: 0L
                    val tx = parts[9].toLongOrNull() ?: 0L
                    return Pair(rx, tx)
                }
            }
            Pair(0L, 0L)
        } catch (_: Exception) {
            Pair(0L, 0L)
        }
    }

    private fun startMetricsPolling() {
        metricsRunnable = object : Runnable {
            override fun run() {
                if (isConnected && view != null) {
                    val (tunDl, tunUl) = readTunBytes()
                    if (lastTunDl == 0L) lastTunDl = tunDl
                    if (lastTunUl == 0L) lastTunUl = tunUl
                    val dlDelta = tunDl - lastTunDl
                    val ulDelta = tunUl - lastTunUl
                    lastTunDl = tunDl
                    lastTunUl = tunUl
                    val dlTotal = SecularVpnService.bytesDownloaded.addAndGet(dlDelta)
                    val ulTotal = SecularVpnService.bytesUploaded.addAndGet(ulDelta)
                    view?.findViewById<TextView>(R.id.dl_speed)?.text = formatBytes(dlTotal)
                    view?.findViewById<TextView>(R.id.ul_speed)?.text = formatBytes(ulTotal)
                    view?.findViewById<TextView>(R.id.session_time)?.text = formatTime(seconds)
                    handler.postDelayed(this, 1000)
                }
            }
        }.also { handler.post(it) }

        seconds = 0
        timerRunnable = object : Runnable {
            override fun run() {
                if (isConnected && view != null) {
                    seconds++
                    handler.postDelayed(this, 1000)
                }
            }
        }.also { handler.post(it) }
    }

    private fun updateUiConnecting() {
        val v = view ?: return
        v.findViewById<TextView>(R.id.status_label)?.text = "Connecting..."
        v.findViewById<TextView>(R.id.status_label)?.setTextColor(resources.getColor(R.color.accent, null))
        v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 0.5f
        animateConnecting(v)
    }

    private fun updateUiConnected(v: View) {
        v.findViewById<TextView>(R.id.status_label)?.text = "Connected"
        v.findViewById<TextView>(R.id.status_label)?.setTextColor(resources.getColor(R.color.accent, null))
        v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 1f
        lastTunDl = 0
        lastTunUl = 0
        SecularVpnService.bytesDownloaded.set(0)
        SecularVpnService.bytesUploaded.set(0)
        animateConnect(v)
    }

    private fun updateUiDisconnected(v: View) {
        v.findViewById<TextView>(R.id.status_label)?.text = "Disconnected"
        v.findViewById<TextView>(R.id.status_label)?.setTextColor(resources.getColor(R.color.text_dim, null))
        v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 0.35f
        animateDisconnect(v)
        v.findViewById<TextView>(R.id.session_time)?.text = "00:00:00"
        v.findViewById<TextView>(R.id.dl_speed)?.text = "0 B"
        v.findViewById<TextView>(R.id.ul_speed)?.text = "0 B"
    }

    private fun animateConnecting(v: View) {
        // Button border → accent
        v.findViewById<FrameLayout>(R.id.connect_btn)?.setBackgroundResource(R.drawable.connect_btn_connecting_bg)

        // White logo → hide
        v.findViewById<View>(R.id.logo_dim)?.visibility = View.GONE

        // Green logo → show with fade
        val logoBright = v.findViewById<View>(R.id.logo_bright)
        logoBright?.visibility = View.VISIBLE
        logoBright?.alpha = 0f
        logoBright?.animate()?.alpha(1f)?.setDuration(500)?.start()

        // Glow → show
        val glow = v.findViewById<View>(R.id.connect_glow)
        glow?.visibility = View.VISIBLE
        glow?.alpha = 0f
        glow?.animate()?.alpha(1f)?.setDuration(600)?.start()

        // Ring → show with spin
        val ring = v.findViewById<View>(R.id.connect_ring)
        ring?.visibility = View.VISIBLE
        ring?.alpha = 1f
        ringAnimator?.cancel()
        ringAnimator = android.animation.ObjectAnimator.ofFloat(ring, "rotation", 0f, 360f).apply {
            duration = 3000
            interpolator = LinearInterpolator()
            repeatCount = android.animation.ValueAnimator.INFINITE
            start()
        }
    }

    private fun animateConnect(v: View) {
        // Button border → green
        v.findViewById<FrameLayout>(R.id.connect_btn)?.setBackgroundResource(R.drawable.connect_btn_connected_bg)

        // White logo → hide
        v.findViewById<View>(R.id.logo_dim)?.visibility = View.GONE

        // Green logo → show with fade
        val logoBright = v.findViewById<View>(R.id.logo_bright)
        logoBright?.visibility = View.VISIBLE
        logoBright?.alpha = 0f
        logoBright?.animate()?.alpha(1f)?.setDuration(500)?.start()

        // Glow → show
        val glow = v.findViewById<View>(R.id.connect_glow)
        glow?.visibility = View.VISIBLE
        glow?.alpha = 0f
        glow?.animate()?.alpha(1f)?.setDuration(600)?.start()

        // Ring → hide (no spinning when connected)
        ringAnimator?.cancel()
        ringAnimator = null
        v.findViewById<View>(R.id.connect_ring)?.visibility = View.GONE

        // Logo pulse (breathing)
        logoBright?.let { lv ->
            lv.scaleX = 1f
            lv.scaleY = 1f
            android.animation.ObjectAnimator.ofFloat(lv, "scaleX", 1f, 1.08f, 1f).apply {
                duration = 2000
                repeatCount = android.animation.ValueAnimator.INFINITE
                start()
            }
            android.animation.ObjectAnimator.ofFloat(lv, "scaleY", 1f, 1.08f, 1f).apply {
                duration = 2000
                repeatCount = android.animation.ValueAnimator.INFINITE
                start()
            }
        }
    }

    private fun animateDisconnect(v: View) {
        // Button border → gray
        v.findViewById<FrameLayout>(R.id.connect_btn)?.setBackgroundResource(R.drawable.connect_btn_bg)

        // Green logo → hide
        v.findViewById<View>(R.id.logo_bright)?.visibility = View.GONE

        // White logo → show
        v.findViewById<View>(R.id.logo_dim)?.visibility = View.VISIBLE

        // Glow → hide
        v.findViewById<View>(R.id.connect_glow)?.animate()?.alpha(0f)?.setDuration(400)?.withEndAction {
            v.findViewById<View>(R.id.connect_glow)?.visibility = View.GONE
        }?.start()

        // Ring → hide
        ringAnimator?.cancel()
        ringAnimator = null
        v.findViewById<View>(R.id.connect_ring)?.visibility = View.GONE
    }

    private fun disconnectVpn() {
        isConnected = false
        ringAnimator?.cancel()
        ringAnimator = null
        lastTunDl = 0
        lastTunUl = 0
        SecularVpnService.bytesDownloaded.set(0)
        SecularVpnService.bytesUploaded.set(0)
        view?.let { updateUiDisconnected(it) }
        timerRunnable?.let { handler.removeCallbacks(it) }
        metricsRunnable?.let { handler.removeCallbacks(it) }
        try {
            val intent = Intent(requireContext(), SecularVpnService::class.java).apply {
                action = SecularVpnService.ACTION_DISCONNECT
            }
            requireContext().startService(intent)
        } catch (_: Exception) {}
    }

    private fun formatTime(totalSeconds: Long): String {
        val h = totalSeconds / 3600
        val m = (totalSeconds % 3600) / 60
        val s = totalSeconds % 60
        return String.format("%02d:%02d:%02d", h, m, s)
    }

    private fun formatBytes(bytes: Long): String = when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> String.format("%.1f KB", bytes / 1024.0)
        else -> String.format("%.1f MB", bytes / (1024.0 * 1024.0))
    }

    override fun onResume() {
        super.onResume()
        view?.let { loadSelectedServer(it) }
        view?.let {
            if (SecularVpnService.isTunnelUp) {
                if (!isConnected) isConnected = true
                updateUiConnected(it)
            } else if (SecularVpnService.isConnecting) {
                updateUiConnecting()
            } else {
                if (isConnected) isConnected = false
                updateUiDisconnected(it)
            }
        }
    }

    override fun onDestroyView() {
        ringAnimator?.cancel()
        ringAnimator = null
        timerRunnable?.let { handler.removeCallbacks(it) }
        metricsRunnable?.let { handler.removeCallbacks(it) }
        super.onDestroyView()
    }
}
