// secular-android/app/src/main/java/com/secular/vpn/ui/LogFragment.kt
// Log screen — real-time VPN/service debug log with copy & clear

package com.secular.vpn.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
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

        // Copy button
        view.findViewById<ImageButton>(R.id.btn_copy_log).setOnClickListener {
            val text = view?.findViewById<TextView>(R.id.log_text)?.text?.toString() ?: ""
            if (text.isNotEmpty()) {
                val clipboard = requireContext().getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                clipboard.setPrimaryClip(ClipData.newPlainText("VPN Log", text))
                Toast.makeText(requireContext(), "Log copied to clipboard", Toast.LENGTH_SHORT).show()
            } else {
                Toast.makeText(requireContext(), "Log is empty", Toast.LENGTH_SHORT).show()
            }
        }

        // Clear button
        view.findViewById<ImageButton>(R.id.btn_clear_log).setOnClickListener {
            synchronized(SecularVpnService.logBuffer) {
                SecularVpnService.logBuffer.clear()
            }
            view?.findViewById<TextView>(R.id.log_text)?.text = ""
            Toast.makeText(requireContext(), "Log cleared", Toast.LENGTH_SHORT).show()
        }

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

                    val text = synchronized(SecularVpnService.logBuffer) {
                        SecularVpnService.logBuffer.joinToString("\n")
                    }
                    logText?.text = text

                    // Auto-scroll to bottom only if user is already near bottom
                    scroll?.let { sv ->
                        val child = sv.getChildAt(0)
                        if (child != null) {
                            val bottom = child.bottom - sv.height - sv.scrollY
                            if (bottom < 500) {
                                sv.post { sv.fullScroll(ScrollView.FOCUS_DOWN) }
                            }
                        }
                    }
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
