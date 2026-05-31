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

    // VPN prepare launcher — shows system "Allow VPN?" dialog
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

        view.findViewById<ImageButton>(R.id.connect_btn).setOnClickListener {
            if (isConnected) disconnectVpn() else connectVpn()
        }

        view.findViewById<LinearLayout>(R.id.server_card).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_serverList)
        }

        view.findViewById<ImageButton>(R.id.nav_servers).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_serverList)
        }
        view.findViewById<FrameLayout>(R.id.nav_home_btn).setOnClickListener { /* already home */ }
        view.findViewById<ImageButton>(R.id.nav_add).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_addServer)
        }

        // Log button
        view.findViewById<ImageButton>(R.id.btn_log).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_log)
        }

        loadSelectedServer(view)
    }

    private fun loadSelectedServer(view: View) {
        lifecycleScope.launch {
            try {
                val servers = repository.loadServers()
                SecularVpnService.addLog("Dashboard: loadSelectedServer — count=${servers.size} names=${servers.map { it.name }}")
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
                    SecularVpnService.addLog("Dashboard: showing server=${server.name}")
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

                // Use selected server, or fall back to first
                val savedName = prefs.getString("selected_server_name", null)
                val server = if (savedName != null) {
                    servers.find { it.name == savedName } ?: servers[0]
                } else {
                    servers[0]
                }
                SecularVpnService.addLog("Connect tapped: server=${server.name} addr=${server.displayAddress}")

                val prepareIntent = try {
                    VpnService.prepare(requireContext())
                } catch (e: Throwable) {
                    SecularVpnService.addLog("VpnService.prepare() failed: ${e.message}")
                    Toast.makeText(requireContext(), "VPN not available: ${e.message}", Toast.LENGTH_SHORT).show()
                    return@launch
                }

                if (prepareIntent != null) {
                    SecularVpnService.addLog("VPN not prepared — showing system dialog")
                    isConnected = true
                    updateUiConnecting()
                    vpnPrepareLauncher.launch(prepareIntent)
                } else {
                    SecularVpnService.addLog("VPN already prepared — starting service directly")
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
                SecularVpnService.addLog("Dashboard: startVpnService server=${server.name} jsonLen=${json.length}")
                val intent = Intent(requireContext(), SecularVpnService::class.java).apply {
                    action = SecularVpnService.ACTION_CONNECT
                    putExtra("server_json", json)
                }
                SecularVpnService.addLog("Dashboard: calling startService...")
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
                    v.findViewById<TextView>(R.id.status_label)?.text = "Connected"
                    v.findViewById<TextView>(R.id.status_label)?.setTextColor(
                        resources.getColor(R.color.accent, null)
                    )
                    v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 1f
                    v.findViewById<ImageButton>(R.id.connect_btn)
                        ?.setBackgroundResource(R.drawable.connect_btn_connected_bg)
                    val ring = v.findViewById<View>(R.id.connect_ring)
                    ring?.visibility = View.VISIBLE
                    android.animation.ObjectAnimator.ofFloat(ring, "rotation", 0f, 360f).apply {
                        duration = 3000
                        interpolator = LinearInterpolator()
                        repeatCount = android.animation.ValueAnimator.INFINITE
                        start()
                    }
                    v.findViewById<View>(R.id.ping_dot)
                        ?.setBackgroundResource(R.drawable.ping_dot_excellent)
                    startMetricsPolling()
                } else if (SecularVpnService.lastError != null) {
                    val v = view ?: return
                    val err = SecularVpnService.lastError ?: "Connection failed"
                    v.findViewById<TextView>(R.id.status_label)?.text = err
                    v.findViewById<TextView>(R.id.status_label)?.setTextColor(
                        resources.getColor(R.color.red, null)
                    )
                    disconnectVpn()
                } else if (SecularVpnService.isConnecting) {
                    handler.postDelayed(this, 1000)
                } else {
                    handler.postDelayed(this, 1000)
                }
            }
        }, 2000)
    }

    private fun startMetricsPolling() {
        metricsRunnable = object : Runnable {
            override fun run() {
                if (isConnected && view != null) {
                    val dl = SecularVpnService.bytesDownloaded.get()
                    val ul = SecularVpnService.bytesUploaded.get()
                    view?.findViewById<TextView>(R.id.dl_speed)?.text = formatBytes(dl)
                    view?.findViewById<TextView>(R.id.ul_speed)?.text = formatBytes(ul)
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
        v.findViewById<TextView>(R.id.status_label)?.setTextColor(
            resources.getColor(R.color.accent, null)
        )
        v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 0.5f
    }

    private fun updateUiDisconnected(v: View) {
        v.findViewById<TextView>(R.id.status_label)?.text = "Disconnected"
        v.findViewById<TextView>(R.id.status_label)?.setTextColor(
            resources.getColor(R.color.text_dim, null)
        )
        v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 0.35f
        v.findViewById<ImageButton>(R.id.connect_btn)
            ?.setBackgroundResource(R.drawable.connect_btn_bg)
        v.findViewById<View>(R.id.connect_ring)?.visibility = View.GONE
        v.findViewById<View>(R.id.ping_dot)?.setBackgroundResource(R.drawable.ping_dot_bg)
        v.findViewById<TextView>(R.id.session_time)?.text = "00:00:00"
        v.findViewById<TextView>(R.id.dl_speed)?.text = "0 B"
        v.findViewById<TextView>(R.id.ul_speed)?.text = "0 B"
    }

    private fun disconnectVpn() {
        isConnected = false
        view?.let { v ->
            v.findViewById<TextView>(R.id.status_label)?.text = "Disconnected"
            v.findViewById<TextView>(R.id.status_label)?.setTextColor(
                resources.getColor(R.color.text_dim, null)
            )
            v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 0.35f
            v.findViewById<ImageButton>(R.id.connect_btn)
                ?.setBackgroundResource(R.drawable.connect_btn_bg)
            v.findViewById<View>(R.id.connect_ring)?.visibility = View.GONE
            v.findViewById<View>(R.id.ping_dot)?.setBackgroundResource(R.drawable.ping_dot_bg)
            v.findViewById<TextView>(R.id.session_time)?.text = "00:00:00"
            v.findViewById<TextView>(R.id.dl_speed)?.text = "0 B"
            v.findViewById<TextView>(R.id.ul_speed)?.text = "0 B"
        }

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
    }

    override fun onDestroyView() {
        timerRunnable?.let { handler.removeCallbacks(it) }
        metricsRunnable?.let { handler.removeCallbacks(it) }
        super.onDestroyView()
    }
}
