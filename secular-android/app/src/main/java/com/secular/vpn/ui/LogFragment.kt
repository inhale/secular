// secular-android/app/src/main/java/com/secular/vpn/ui/LogFragment.kt
// Log screen — real-time VPN/service debug log with filtering, selection, copy & clear

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
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.secular.vpn.R
import com.secular.vpn.SecularVpnService

data class LogEntry(
    val id: Int,
    val time: String,
    val msg: String,
    val level: String // "ok", "warn", "err"
)

class LogFragment : Fragment() {

    private val handler = Handler(Looper.getMainLooper())
    private var pollRunnable: Runnable? = null

    private val allEntries = mutableListOf<LogEntry>()
    private val selectedIds = mutableSetOf<Int>()
    private var nextId = 0
    private var lastBufferIndex = 0
    private var isAtBottom = true

    private lateinit var adapter: LogAdapter
    private lateinit var recyclerView: RecyclerView
    private lateinit var logLayout: LinearLayoutManager
    private lateinit var scrollHint: TextView
    private lateinit var selectionBar: LinearLayout
    private lateinit var selInfo: TextView
    private lateinit var filterPopupView: LinearLayout
    private lateinit var filterBadge: View

    private var filterOk = true
    private var filterWarn = true
    private var filterErr = true

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_log, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)

        recyclerView = view.findViewById(R.id.log_recycler)
        scrollHint = view.findViewById(R.id.scroll_hint)
        selectionBar = view.findViewById(R.id.selection_bar)
        selInfo = view.findViewById(R.id.sel_info)
        filterBadge = view.findViewById(R.id.filter_badge)

        // Setup RecyclerView
        logLayout = LinearLayoutManager(requireContext())
        recyclerView.layoutManager = logLayout
        adapter = LogAdapter(
            onItemClick = { entry -> toggleSelection(entry) },
            isSelected = { entry -> selectedIds.contains(entry.id) }
        )
        recyclerView.adapter = adapter

        // Detect scroll position for "new entries" hint
        recyclerView.addOnScrollListener(object : RecyclerView.OnScrollListener() {
            override fun onScrolled(rv: RecyclerView, dx: Int, dy: Int) {
                val lastVisible = logLayout.findLastCompletelyVisibleItemPosition()
                isAtBottom = lastVisible >= adapter.itemCount - 2 || adapter.itemCount <= 1
                scrollHint.visibility = if (isAtBottom || adapter.itemCount == 0) View.GONE else View.VISIBLE
            }
        })

        // Setup filter popup content
        filterPopupView = LayoutInflater.from(requireContext()).inflate(R.layout.filter_popup, null) as LinearLayout
        setupFilterActions(filterPopupView)

        // Filter button
        view.findViewById<ImageButton>(R.id.btn_filter_log).setOnClickListener {
            showFilterPopup(it)
        }

        // Copy all button
        view.findViewById<ImageButton>(R.id.btn_copy_log).setOnClickListener {
            copyAll()
        }

        // Clear button
        view.findViewById<ImageButton>(R.id.btn_clear_log).setOnClickListener {
            clearLog()
        }

        // Copy selected button
        view.findViewById<ImageButton>(R.id.btn_copy_selected).setOnClickListener {
            copySelected()
        }

        // Clear selection button
        view.findViewById<ImageButton>(R.id.btn_clear_selection).setOnClickListener {
            clearSelection()
        }

        // Scroll hint click → scroll to bottom
        scrollHint.setOnClickListener {
            recyclerView.smoothScrollToPosition(adapter.itemCount - 1)
        }

        // Bottom nav
        view.findViewById<ImageButton>(R.id.nav_home_btn).setOnClickListener {
            findNavController().popBackStack()
        }
        view.findViewById<ImageButton>(R.id.nav_add).setOnClickListener {
            try { findNavController().navigate(R.id.action_logFragment_to_addServer) } catch (_: Exception) { findNavController().popBackStack() }
        }

        startPolling()
    }

    private fun setupFilterActions(popup: LinearLayout) {
        // OK / Info
        popup.findViewById<View>(R.id.filter_ok).setOnClickListener {
            filterOk = !filterOk
            updateFilterRow(popup, R.id.filter_ok, filterOk)
            applyFilter()
        }
        // Warn
        popup.findViewById<View>(R.id.filter_warn).setOnClickListener {
            filterWarn = !filterWarn
            updateFilterRow(popup, R.id.filter_warn, filterWarn)
            applyFilter()
        }
        // Err
        popup.findViewById<View>(R.id.filter_err).setOnClickListener {
            filterErr = !filterErr
            updateFilterRow(popup, R.id.filter_err, filterErr)
            applyFilter()
        }
    }

    private fun updateFilterRow(popup: View, rowId: Int, active: Boolean) {
        val row = popup.findViewById<ViewGroup>(rowId)
        val checkTv = row.getChildAt(row.childCount - 1) as? TextView
        checkTv?.text = if (active) "✓" else ""
    }

    private fun showFilterPopup(anchor: View) {
        val popup = android.widget.PopupWindow(
            filterPopupView,
            (160 * resources.displayMetrics.density).toInt(),
            ViewGroup.LayoutParams.WRAP_CONTENT,
            true
        )

        // Dismiss when tapping outside (matches mockup behavior)
        popup.isOutsideTouchable = true
        popup.setBackgroundDrawable(android.graphics.drawable.ColorDrawable(android.graphics.Color.TRANSPARENT))

        // Position the popup below the button
        val location = IntArray(2)
        anchor.getLocationOnScreen(location)
        val popupWidth = (160 * resources.displayMetrics.density).toInt()
        val x = (location[0] + anchor.width - popupWidth).coerceAtLeast(0)
        val y = location[1] + anchor.height + 8
        popup.showAtLocation(anchor, 0, x, y)
    }

    private fun applyFilter() {
        val filtered = allEntries.filter { entry ->
            when (entry.level) {
                "ok" -> filterOk
                "info" -> filterOk  // info grouped with ok toggle
                "warn" -> filterWarn
                "err" -> filterErr
                else -> true
            }
        }
        adapter.updateData(filtered)
        if (isAtBottom && filtered.isNotEmpty()) {
            recyclerView.scrollToPosition(filtered.size - 1)
        }
        // Update filter badge
        val anyOff = !filterOk || !filterWarn || !filterErr
        filterBadge.visibility = if (anyOff) View.VISIBLE else View.GONE
    }

    private fun toggleSelection(entry: LogEntry) {
        if (selectedIds.contains(entry.id)) {
            selectedIds.remove(entry.id)
        } else {
            selectedIds.add(entry.id)
        }
        adapter.notifyItemChanged(allEntries.indexOf(entry))
        updateSelectionBar()
    }

    private fun updateSelectionBar() {
        if (selectedIds.isNotEmpty()) {
            selInfo.text = "${selectedIds.size} line${if (selectedIds.size > 1) "s" else ""} selected"
            selectionBar.visibility = View.VISIBLE
        } else {
            selectionBar.visibility = View.GONE
        }
    }

    private fun clearSelection() {
        selectedIds.clear()
        adapter.notifyDataSetChanged()
        updateSelectionBar()
    }

    private fun copyAll() {
        val visible = adapter.getVisibleLines()
        if (visible.isEmpty()) {
            Toast.makeText(requireContext(), "Log is empty", Toast.LENGTH_SHORT).show()
            return
        }
        val text = visible.joinToString("\n") { e ->
            "${e.time} [${e.level.uppercase()}] ${e.msg}"
        }
        doCopy(text, "Log copied to clipboard")
    }

    private fun copySelected() {
        val selected = allEntries.filter { selectedIds.contains(it.id) }
        if (selected.isEmpty()) return
        val text = selected.joinToString("\n") { e ->
            "${e.time} [${e.level.uppercase()}] ${e.msg}"
        }
        doCopy(text, "Selected lines copied")
    }

    private fun doCopy(text: String, toast: String) {
        val clipboard = requireContext().getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("VPN Log", text))
        Toast.makeText(requireContext(), toast, Toast.LENGTH_SHORT).show()
    }

    private fun clearLog() {
        synchronized(SecularVpnService.logBuffer) {
            SecularVpnService.logBuffer.clear()
        }
        allEntries.clear()
        selectedIds.clear()
        lastBufferIndex = 0
        adapter.updateData(emptyList())
        Toast.makeText(requireContext(), "Log cleared", Toast.LENGTH_SHORT).show()
    }

    /**
     * Detect log level from message content.
     * Returns "err" for errors, "warn" for warnings, "info" for informational messages,
     * "ok" for positive status/confirmation messages.
     * Matches mockup: ok=green, info=blue, warn=yellow, err=red.
     */
    private fun detectLevel(msg: String): String {
        val lower = msg.lowercase()
        return when {
            lower.contains("error") || lower.contains("err ") || lower.contains("failed") ||
                lower.contains("fatal") || lower.contains("denied") || lower.contains("crash") -> "err"
            lower.contains("warn") || lower.contains("spike") || lower.contains("retransmit") ||
                lower.contains("stale") || lower.contains("timeout") || lower.contains("brief") ||
                lower.contains("jitter") -> "warn"
            // Positive status confirmations get "ok"
            lower.contains("stable") || lower.contains("acknowledged") || lower.contains("completed") ||
                lower.contains("connected") || lower.contains("established") || lower.contains("restored") ||
                lower.contains("renewed") || lower.contains("negotiation complete") -> "ok"
            // Everything else is informational
            else -> "info"
        }
    }

    private fun startPolling() {
        val tsFormat = java.text.SimpleDateFormat("HH:mm:ss.SSS", java.util.Locale.US)

        pollRunnable = object : Runnable {
            override fun run() {
                if (view == null) return
                try {
                    // Read new lines from the service buffer
                    val newLines = synchronized(SecularVpnService.logBuffer) {
                        val buffer = SecularVpnService.logBuffer
                        if (lastBufferIndex < buffer.size) {
                            val lines = buffer.subList(lastBufferIndex, buffer.size).toList()
                            lastBufferIndex = buffer.size
                            lines
                        } else {
                            emptyList()
                        }
                    }

                    if (newLines.isNotEmpty()) {
                        val now = java.util.Date()
                        for (line in newLines) {
                            val time = tsFormat.format(now)
                            val level = detectLevel(line)
                            val entry = LogEntry(id = nextId++, time = time, msg = line, level = level)
                            allEntries.add(entry)
                            // Cap buffer to prevent OOM
                            if (allEntries.size > 500) {
                                val removed = allEntries.removeAt(0)
                                selectedIds.remove(removed.id)
                            }
                        }
                        applyFilter()
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

// ── RecyclerView Adapter ──

class LogAdapter(
    private val onItemClick: (LogEntry) -> Unit,
    private val isSelected: (LogEntry) -> Boolean
) : RecyclerView.Adapter<LogAdapter.ViewHolder>() {

    private var entries = listOf<LogEntry>()

    class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val timeTv: TextView = view.findViewById(R.id.log_time)
        val levelTv: TextView = view.findViewById(R.id.log_level)
        val msgTv: TextView = view.findViewById(R.id.log_msg)
        val root: View = view
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
        val view = LayoutInflater.from(parent.context)
            .inflate(R.layout.item_log_line, parent, false)
        return ViewHolder(view)
    }

    override fun onBindViewHolder(holder: ViewHolder, position: Int) {
        val entry = entries[position]
        holder.timeTv.text = entry.time

        val levelLabel = when (entry.level) {
            "ok" -> "OK"
            "info" -> "INFO"
            "warn" -> "WARN"
            "err" -> "ERR"
            else -> "OK"
        }
        holder.levelTv.text = levelLabel
        holder.levelTv.setTextColor(when (entry.level) {
            "ok" -> 0xFF00FF66.toInt()
            "info" -> 0xFF5B9AFF.toInt()
            "warn" -> 0xFFFFD93D.toInt()
            "err" -> 0xFFFF4D4D.toInt()
            else -> 0xFF00FF66.toInt()
        })
        holder.msgTv.text = entry.msg

        // Selection highlight: green tint + brighter text for selected lines
        if (isSelected(entry)) {
            holder.root.setBackgroundColor(0x1400FF66.toInt())
            holder.msgTv.setTextColor(0xFFFFFFFF.toInt())
        } else {
            holder.root.setBackgroundColor(0x00000000.toInt())
            holder.msgTv.setTextColor(0xFF8A8A8A.toInt())
        }

        holder.root.setOnClickListener { onItemClick(entry) }
    }

    override fun getItemCount() = entries.size

    fun updateData(newEntries: List<LogEntry>) {
        entries = newEntries
        notifyDataSetChanged()
    }

    fun getVisibleLines(): List<LogEntry> = entries
}
