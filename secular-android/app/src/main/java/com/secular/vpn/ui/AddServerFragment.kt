// secular-android/app/src/main/java/com/secular/vpn/ui/AddServerFragment.kt
// Add server screen — 4 methods: link, QR, file upload, manual

package com.secular.vpn.ui

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.navigation.fragment.findNavController
import com.secular.vpn.R
import com.secular.vpn.SecularVpnService
import com.secular.vpn.data.DeepLinkParser
import com.secular.vpn.data.TomlFileParser
import com.secular.vpn.data.ServerProfile
import com.secular.vpn.data.ServersRepository
import kotlinx.coroutines.launch

class AddServerFragment : Fragment() {

    private lateinit var repository: ServersRepository
    private lateinit var linkInput: EditText
    private lateinit var prefs: SharedPreferences

    private val tomlPickerLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            result.data?.data?.let { uri -> parseTomlFile(uri) }
        }
    }

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, arguments: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_add_server, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)
        repository = ServersRepository(requireContext())
        prefs = requireContext().getSharedPreferences("secular_vpn_prefs", Context.MODE_PRIVATE)

        linkInput = view.findViewById(R.id.field_link)

        view.findViewById<Button>(R.id.btn_add_link).setOnClickListener { addFromLink() }
        view.findViewById<Button>(R.id.btn_scan_qr).setOnClickListener {
            Toast.makeText(requireContext(), "QR Scanner coming soon", Toast.LENGTH_SHORT).show()
        }
        view.findViewById<LinearLayout>(R.id.btn_upload_config).setOnClickListener {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
                putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("application/octet-stream", "text/plain"))
            }
            tomlPickerLauncher.launch(intent)
        }
        view.findViewById<Button>(R.id.btn_manual_setup).setOnClickListener {
            val bundle = Bundle().apply { putInt("serverIndex", -1) }
            findNavController().navigate(R.id.action_addServer_to_serverConfig, bundle)
        }

        // Bottom nav
        view.findViewById<ImageButton>(R.id.nav_servers).setOnClickListener {
            try { findNavController().popBackStack() } catch (_: Exception) {}
        }
        view.findViewById<FrameLayout>(R.id.nav_home_btn).setOnClickListener {
            try { findNavController().popBackStack() } catch (_: Exception) {}
        }
    }

    private fun addFromLink() {
        val link = linkInput.text.toString().trim()
        if (link.isEmpty()) {
            linkInput.error = "Please enter a link"
            linkInput.requestFocus()
            return
        }

        SecularVpnService.addLog("AddServer: addFromLink() → parsing link")
        val profile = DeepLinkParser.parse(link)
        if (profile != null) {
            SecularVpnService.addLog("AddServer: parsed OK name=${profile.name} host=${profile.hostname}")
            lifecycleScope.launch {
                val idx = repository.addServer(profile)
                SecularVpnService.addLog("AddServer: added server idx=$idx, navigating to server list")
                prefs.edit().putString("selected_server_name", profile.name).apply()
                // Navigate: pop back to dashboard, then go to server list
                findNavController().popBackStack(R.id.dashboardFragment, false)
                findNavController().navigate(R.id.action_dashboard_to_serverList)
            }
        } else {
            SecularVpnService.addLog("AddServer: FAILED to parse link")
            Toast.makeText(requireContext(), "Invalid link format. Check Log screen.", Toast.LENGTH_LONG).show()
        }
    }

    private fun parseTomlFile(uri: Uri) {
        lifecycleScope.launch {
            try {
                val inputStream = requireContext().contentResolver.openInputStream(uri)
                if (inputStream != null) {
                    val profile = TomlFileParser.parse(inputStream)
                    inputStream.close()
                    if (profile != null) {
                        SecularVpnService.addLog("AddServer: TOML parsed name=${profile.name}")
                        repository.addServer(profile)
                        prefs.edit().putString("selected_server_name", profile.name).apply()
                        findNavController().popBackStack(R.id.dashboardFragment, false)
                        findNavController().navigate(R.id.action_dashboard_to_serverList)
                    } else {
                        Toast.makeText(requireContext(), "Invalid TOML config", Toast.LENGTH_LONG).show()
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(requireContext(), "Error: ${e.message}", Toast.LENGTH_LONG).show()
            }
        }
    }
}
