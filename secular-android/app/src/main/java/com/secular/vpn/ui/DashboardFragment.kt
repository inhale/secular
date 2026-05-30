// secular-android/app/src/main/java/com/secular/vpn/ui/DashboardFragment.kt
// Dashboard screen — main VPN connection screen

package com.secular.vpn.ui

import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.widget.*
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.navigation.fragment.findNavController
import com.secular.vpn.R
import com.secular.vpn.SecularVpnService
import com.secular.vpn.data.ServersRepository
import kotlinx.coroutines.launch

class DashboardFragment : Fragment() {

    private lateinit var repository: ServersRepository
    private var isConnected = false
    private var seconds = 0L
    private val handler = Handler(Looper.getMainLooper())
    private var timerRunnable: Runnable? = null
    private var metricsRunnable: Runnable? = null

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_dashboard, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)
        repository = ServersRepository(requireContext())

        view.findViewById<ImageButton>(R.id.connect_btn).setOnClickListener {
            if (isConnected) disconnectVpn() else connectVpn()
        }

        view.findViewById<LinearLayout>(R.id.server_card).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_serverList)
        }

        view.findViewById<ImageButton>(R.id.nav_servers).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_serverList)
        }
        view.findViewById<ImageButton>(R.id.nav_home).setOnClickListener {
            // Already on dashboard
        }
        view.findViewById<ImageButton>(R.id.nav_add).setOnClickListener {
            findNavController().navigate(R.id.action_dashboard_to_addServer)
        }

        loadSelectedServer(view)
    }

    private fun loadSelectedServer(view: View) {
        lifecycleScope.launch {
            val servers = repository.loadServers()
            val nameTv = view.findViewById<TextView>(R.id.server_card_name)
            val metaTv = view.findViewById<TextView>(R.id.server_card_meta)
            val pingDot = view.findViewById<View>(R.id.ping_dot)

            if (servers.isNotEmpty()) {
                val server = servers[0]
                nameTv.text = server.name
                metaTv.text = "TrustTunnel · ${server.displayAddress}"
                pingDot.setBackgroundResource(R.drawable.ping_dot_bg)
            } else {
                nameTv.text = "No server selected"
                metaTv.text = "Tap to add a server"
                pingDot.setBackgroundResource(R.drawable.ping_dot_bg)
            }
        }
    }

    private fun connectVpn() {
        lifecycleScope.launch {
            val servers = repository.loadServers()
            if (servers.isEmpty()) {
                Toast.makeText(requireContext(), "Add a server first", Toast.LENGTH_SHORT).show()
                return@launch
            }

            val server = servers[0]
            val intent = Intent(requireContext(), SecularVpnService::class.java).apply {
                action = SecularVpnService.ACTION_CONNECT
                putExtra("server_json", com.google.gson.Gson().toJson(server))
            }
            requireContext().startService(intent)

            // Update UI to connecting state
            isConnected = true
            val v = view ?: return@launch

            v.findViewById<TextView>(R.id.status_label).text = "Connecting..."
            v.findViewById<TextView>(R.id.status_label).setTextColor(
                resources.getColor(R.color.accent, null)
            )

            // Start timer
            seconds = 0
            timerRunnable = object : Runnable {
                override fun run() {
                    if (isConnected && view != null) {
                        seconds++
                        view?.findViewById<TextView>(R.id.session_time)?.text = formatTime(seconds)
                        handler.postDelayed(this, 1000)
                    }
                }
            }
            handler.post(timerRunnable!!)

            // Start polling real metrics from service
            startMetricsPolling()

            // After 2s, update status based on actual tunnel state
            handler.postDelayed({
                if (SecularVpnService.isTunnelUp) {
                    v.findViewById<TextView>(R.id.status_label)?.text = "Connected"
                    v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 1f
                    val connectBtn = v.findViewById<ImageButton>(R.id.connect_btn)
                    connectBtn.setBackgroundResource(R.drawable.connect_btn_connected_bg)
                    val connectRing = v.findViewById<View>(R.id.connect_ring)
                    connectRing.visibility = View.VISIBLE
                    android.animation.ObjectAnimator.ofFloat(connectRing, "rotation", 0f, 360f).apply {
                        duration = 3000
                        interpolator = LinearInterpolator()
                        repeatCount = android.animation.ValueAnimator.INFINITE
                        start()
                    }
                    v.findViewById<View>(R.id.ping_dot)?.setBackgroundResource(R.drawable.ping_dot_excellent)
                } else {
                    val err = SecularVpnService.lastError ?: "Connection failed"
                    v.findViewById<TextView>(R.id.status_label)?.text = err
                    v.findViewById<TextView>(R.id.status_label)?.setTextColor(
                        resources.getColor(R.color.red, null)
                    )
                    v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 0.35f
                    disconnectVpn()
                }
            }, 3000)
        }
    }

    private fun startMetricsPolling() {
        metricsRunnable = object : Runnable {
            override fun run() {
                if (isConnected && view != null) {
                    val dl = SecularVpnService.bytesDownloaded.get()
                    val ul = SecularVpnService.bytesUploaded.get()
                    view?.findViewById<TextView>(R.id.dl_speed)?.text = formatBytes(dl)
                    view?.findViewById<TextView>(R.id.ul_speed)?.text = formatBytes(ul)
                    handler.postDelayed(this, 1000)
                }
            }
        }
        handler.post(metricsRunnable!!)
    }

    private fun disconnectVpn() {
        isConnected = false
        val v = view ?: return

        v.findViewById<TextView>(R.id.status_label)?.text = "Disconnected"
        v.findViewById<TextView>(R.id.status_label)?.setTextColor(
            resources.getColor(R.color.text_dim, null)
        )

        v.findViewById<LinearLayout>(R.id.metrics_container)?.alpha = 0.35f

        v.findViewById<ImageButton>(R.id.connect_btn)?.setBackgroundResource(R.drawable.connect_btn_bg)
        v.findViewById<View>(R.id.connect_ring)?.visibility = View.GONE

        timerRunnable?.let { handler.removeCallbacks(it) }
        metricsRunnable?.let { handler.removeCallbacks(it) }

        v.findViewById<TextView>(R.id.session_time)?.text = "00:00:00"
        v.findViewById<TextView>(R.id.dl_speed)?.text = "0 B"
        v.findViewById<TextView>(R.id.ul_speed)?.text = "0 B"
        v.findViewById<View>(R.id.ping_dot)?.setBackgroundResource(R.drawable.ping_dot_bg)

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

    private fun formatBytes(bytes: Long): String {
        return when {
            bytes < 1024 -> "$bytes B"
            bytes < 1024 * 1024 -> String.format("%.1f KB", bytes / 1024.0)
            else -> String.format("%.1f MB", bytes / (1024.0 * 1024.0))
        }
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
