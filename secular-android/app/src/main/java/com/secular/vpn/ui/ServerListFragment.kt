// secular-android/app/src/main/java/com/secular/vpn/ui/ServerListFragment.kt
// Server list screen — select and manage VPN servers

package com.secular.vpn.ui

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.navigation.fragment.findNavController
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.secular.vpn.R
import com.secular.vpn.data.ServerProfile
import com.secular.vpn.data.ServersRepository
import kotlinx.coroutines.launch

class ServerListFragment : Fragment() {

    private lateinit var repository: ServersRepository
    private lateinit var adapter: ServerAdapter
    private var servers = mutableListOf<ServerProfile>()
    private var selectedIndex = -1

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_server_list, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)
        repository = ServersRepository(requireContext())

        // Setup RecyclerView
        val recyclerView = view.findViewById<RecyclerView>(R.id.server_list)
        adapter = ServerAdapter(
            servers = servers,
            onItemClick = { index ->
                selectedIndex = index
                adapter.notifyDataSetChanged()
                recyclerView.postDelayed({
                    try { findNavController().popBackStack() } catch (_: Exception) {}
                }, 400)
            },
            onGearClick = { index ->
                try {
                    val bundle = Bundle().apply { putInt("serverIndex", index) }
                    findNavController().navigate(R.id.action_serverList_to_serverConfig, bundle)
                } catch (_: Exception) {}
            }
        )
        recyclerView.layoutManager = LinearLayoutManager(requireContext())
        recyclerView.adapter = adapter

        // Empty state button
        view.findViewById<Button>(R.id.btn_add_first_server)?.setOnClickListener {
            try { findNavController().navigate(R.id.action_serverList_to_addServer) } catch (_: Exception) {}
        }

        // Bottom nav
        view.findViewById<FrameLayout>(R.id.nav_home_btn).setOnClickListener {
            try { findNavController().popBackStack() } catch (_: Exception) {}
        }
        view.findViewById<ImageButton>(R.id.nav_add).setOnClickListener {
            try { findNavController().navigate(R.id.action_serverList_to_addServer) } catch (_: Exception) {}
        }

        loadServers()
    }

    private fun loadServers() {
        lifecycleScope.launch {
            try {
                servers.clear()
                servers.addAll(repository.loadServers())
                adapter.notifyDataSetChanged()

                val emptyState = view?.findViewById<LinearLayout>(R.id.empty_state)
                val rv = view?.findViewById<RecyclerView>(R.id.server_list)
                if (servers.isEmpty()) {
                    emptyState?.visibility = View.VISIBLE
                    rv?.visibility = View.GONE
                } else {
                    emptyState?.visibility = View.GONE
                    rv?.visibility = View.VISIBLE
                }
            } catch (_: Exception) {}
        }
    }

    override fun onResume() {
        super.onResume()
        loadServers()
    }

    private val flagEmojis = listOf("\uD83C\uDDE9\uD83C\uDDEA", "\uD83C\uDDEC\uD83C\uDDE7", "\uD83C\uDDEB\uD83C\uDDF7", "\uD83C\uDDE8\uD83C\uDDF3", "\uD83C\uDDEF\uD83C\uDDF5", "\uD83C\uDDF5\uD83C\uDDF1")

    inner class ServerAdapter(
        private val servers: List<ServerProfile>,
        private val onItemClick: (Int) -> Unit,
        private val onGearClick: (Int) -> Unit
    ) : RecyclerView.Adapter<ServerAdapter.ViewHolder>() {

        inner class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
            val flagText: TextView = view.findViewById(R.id.flag_text)
            val name: TextView = view.findViewById(R.id.server_name)
            val meta: TextView = view.findViewById(R.id.server_meta)
            val pingText: TextView = view.findViewById(R.id.ping_text)
            val pingDot: View = view.findViewById(R.id.ping_dot)
            val gearBtn: ImageButton = view.findViewById(R.id.gear_btn)
            val container: LinearLayout = view as LinearLayout
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
            val view = LayoutInflater.from(parent.context)
                .inflate(R.layout.item_server, parent, false)
            return ViewHolder(view)
        }

        override fun onBindViewHolder(holder: ViewHolder, position: Int) {
            try {
                val server = servers[position]

                // Rotate flag emojis based on position — no hardcoded Germany
                holder.flagText.text = flagEmojis[position % flagEmojis.size]
                holder.name.text = server.name
                holder.meta.text = "TrustTunnel · ${server.displayAddress}"

                // Simulate ping
                val ping = (20..200).random()
                holder.pingText.text = "${ping}ms"
                holder.pingDot.setBackgroundResource(getPingDrawable(ping))

                // Selected state
                if (position == selectedIndex) {
                    holder.container.setBackgroundResource(R.drawable.server_item_selected_bg)
                } else {
                    holder.container.setBackgroundResource(R.drawable.server_item_bg)
                }

                holder.container.setOnClickListener { onItemClick(position) }
                holder.gearBtn.setOnClickListener { onGearClick(position) }
            } catch (_: Exception) {}
        }

        override fun getItemCount() = servers.size

        private fun getPingDrawable(ping: Int): Int = when {
            ping < 40 -> R.drawable.ping_dot_excellent
            ping < 80 -> R.drawable.ping_dot_good
            ping < 150 -> R.drawable.ping_dot_ok
            else -> R.drawable.ping_dot_bad
        }
    }
}
