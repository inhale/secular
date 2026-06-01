// secular-android/app/src/main/java/com/secular/vpn/ui/ServerListFragment.kt
// Server list screen — select and manage VPN servers

package com.secular.vpn.ui

import android.content.Context
import android.content.SharedPreferences
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
import com.secular.vpn.SecularVpnService
import com.secular.vpn.data.ServerProfile
import com.secular.vpn.data.ServersRepository
import kotlinx.coroutines.launch

class ServerListFragment : Fragment() {

    private lateinit var repository: ServersRepository
    private lateinit var adapter: ServerAdapter
    private val servers = mutableListOf<ServerProfile>()
    private var selectedIndex = -1
    private lateinit var prefs: SharedPreferences

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_server_list, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)
        repository = ServersRepository(requireContext())
        prefs = requireContext().getSharedPreferences("secular_vpn_prefs", Context.MODE_PRIVATE)

        val recyclerView = view.findViewById<RecyclerView>(R.id.server_list)
        adapter = ServerAdapter(
            servers = servers,
            selectedIndex = selectedIndex,
            onItemClick = { index ->
                selectedIndex = index
                adapter.updateSelected(index)
                val server = servers.getOrNull(index)
                if (server != null) {
                    prefs.edit().putString("selected_server_name", server.name).apply()
                }
                // Pop back to dashboard after selection
                view.postDelayed({
                    try { findNavController().popBackStack() } catch (_: Exception) {}
                }, 300)
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

        view.findViewById<Button>(R.id.btn_add_first_server)?.setOnClickListener {
            try { findNavController().navigate(R.id.action_serverList_to_addServer) } catch (_: Exception) {}
        }

        view.findViewById<ImageButton>(R.id.nav_log).setOnClickListener {
            try { findNavController().navigate(R.id.action_serverList_to_log) } catch (_: Exception) {}
        }
        view.findViewById<FrameLayout>(R.id.nav_home_btn).setOnClickListener {
            try { findNavController().popBackStack() } catch (_: Exception) {}
        }
        view.findViewById<ImageButton>(R.id.nav_add).setOnClickListener {
            try { findNavController().navigate(R.id.action_serverList_to_addServer) } catch (_: Exception) {}
        }
    }

    override fun onResume() {
        super.onResume()
        loadServers()
    }

    private fun loadServers() {
        SecularVpnService.addLog("ServerList: loadServers() START")
        lifecycleScope.launch {
            try {
                val loaded = repository.loadServers()
                SecularVpnService.addLog("ServerList: loadServers() — loaded=${loaded.size} names=${loaded.map { it.name }}")
                if (loaded.isEmpty()) {
                    val emptyState = view?.findViewById<LinearLayout>(R.id.empty_state)
                    val rv = view?.findViewById<RecyclerView>(R.id.server_list)
                    emptyState?.visibility = View.VISIBLE
                    rv?.visibility = View.GONE
                    adapter.updateList(emptyList(), -1)
                    return@launch
                }
                servers.clear()
                servers.addAll(loaded)

                // Restore selection
                val savedName = prefs.getString("selected_server_name", null)
                SecularVpnService.addLog("ServerList: savedName=$savedName servers=${servers.map { it.name }}")
                if (savedName != null) {
                    val idx = servers.indexOfFirst { it.name == savedName }
                    if (idx >= 0) selectedIndex = idx
                } else if (servers.isNotEmpty()) {
                    selectedIndex = 0
                }
                SecularVpnService.addLog("ServerList: calling updateList size=${servers.size} selectedIndex=$selectedIndex")

                adapter.updateList(servers, selectedIndex)

                val emptyState = view?.findViewById<LinearLayout>(R.id.empty_state)
                val rv = view?.findViewById<RecyclerView>(R.id.server_list)
                emptyState?.visibility = View.GONE
                rv?.visibility = View.VISIBLE
                SecularVpnService.addLog("ServerList: loadServers() DONE — showing ${servers.size} servers")
            } catch (e: Throwable) {
                SecularVpnService.addLog("ServerList: loadServers ERROR: ${e.javaClass.simpleName}: ${e.message}")
            }
        }
    }

    inner class ServerAdapter(
        private val servers: MutableList<ServerProfile>,
        private var selectedIndex: Int,
        private val onItemClick: (Int) -> Unit,
        private val onGearClick: (Int) -> Unit
    ) : RecyclerView.Adapter<ServerAdapter.ViewHolder>() {

        inner class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
            val defaultBadge: TextView = view.findViewById(R.id.default_badge)
            val name: TextView = view.findViewById(R.id.server_name)
            val meta: TextView = view.findViewById(R.id.server_meta)
            val gearBtn: ImageButton = view.findViewById(R.id.gear_btn)
            val container: LinearLayout = view as LinearLayout
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
            val view = LayoutInflater.from(parent.context).inflate(R.layout.item_server, parent, false)
            return ViewHolder(view)
        }

        override fun onBindViewHolder(holder: ViewHolder, position: Int) {
            try {
                val server = servers[position]
                holder.name.text = server.name
                holder.meta.text = "TrustTunnel · ${server.displayAddress}"
                // Show DEFAULT badge for selected server
                holder.defaultBadge.visibility = if (position == selectedIndex) View.VISIBLE else View.GONE
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

        fun updateList(newServers: List<ServerProfile>, newSelected: Int) {
            val copy = ArrayList(newServers)
            servers.clear()
            servers.addAll(copy)
            selectedIndex = newSelected
            notifyDataSetChanged()
        }

        fun updateSelected(newSelected: Int) {
            val old = selectedIndex
            selectedIndex = newSelected
            if (old >= 0 && old < servers.size) notifyItemChanged(old)
            if (newSelected >= 0 && newSelected < servers.size) notifyItemChanged(newSelected)
        }
    }
}
