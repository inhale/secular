// secular-android/app/src/main/java/com/secular/vpn/ui/LogFragment.kt
// Log screen — real-time VPN/service debug log

package com.secular.vpn.ui

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.fragment.app.Fragment
import androidx.navigation.fragment.findNavController
import com.secular.vpn.R
import com.secular.vpn.SecularVpnService

class LogFragment : Fragment() {

    private val handler = Handler(Looper.getMainLooper())
    private var pollRunnable: Runnable? = null

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_log, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)

        // Bottom nav
        view.findViewById<ImageButton>(R.id.nav_servers).setOnClickListener {
            findNavController().popBackStack()
        }
        view.findViewById<ImageButton>(R.id.nav_home).setOnClickListener {
            findNavController().popBackStack()
        }
        view.findViewById<ImageButton>(R.id.nav_add).setOnClickListener {
            try { findNavController().navigate(R.id.action_logFragment_to_addServer) } catch (_: Exception) { findNavController().popBackStack() }
        }

        startPolling()
    }

    private fun startPolling() {
        pollRunnable = object : Runnable {
            override fun run() {
                if (view == null) return
                try {
                    val logText = view?.findViewById<TextView>(R.id.log_text)
                    val scroll = view?.findViewById<ScrollView>(R.id.log_scroll)

                    synchronized(SecularVpnService.logBuffer) {
                        logText?.text = SecularVpnService.logBuffer.joinToString("\n")
                    }

                    // Auto-scroll to bottom
                    scroll?.post { scroll.fullScroll(ScrollView.FOCUS_DOWN) }
                } catch (_: Exception) {}

                handler.postDelayed(this, 500)
            }
        }.also { handler.post(it) }
    }

    override fun onDestroyView() {
        pollRunnable?.let { handler.removeCallbacks(it) }
        super.onDestroyView()
    }
}
