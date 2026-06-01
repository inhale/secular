// secular-android/app/src/main/java/com/secular/vpn/ui/ServerConfigFragment.kt
// Server configuration screen — edit/create server details

package com.secular.vpn.ui

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.os.Bundle
import android.text.InputType
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.navigation.fragment.findNavController
import com.secular.vpn.R
import com.secular.vpn.SecularVpnService
import com.secular.vpn.data.ServerProfile
import com.secular.vpn.data.ServersRepository
import kotlinx.coroutines.launch

class ServerConfigFragment : Fragment() {

    private lateinit var repository: ServersRepository
    private var serverIndex = -1
    private var existingServer: ServerProfile? = null
    private var certFilePath: String? = null
    private var isSaving = false
    private lateinit var prefs: SharedPreferences

    // Form fields
    private lateinit var fieldName: EditText
    private lateinit var fieldIp: EditText
    private lateinit var fieldHostname: EditText
    private lateinit var fieldUsername: EditText
    private lateinit var fieldPassword: EditText
    private lateinit var fieldDns: EditText
    private lateinit var fieldIpv6: CheckBox
    private lateinit var protocolSpinner: Spinner
    private lateinit var certFileName: TextView
    private lateinit var btnTogglePassword: ImageButton
    private lateinit var btnDelete: ImageButton
    private lateinit var headerTitle: TextView

    private var passwordVisible = false

    private val certPickerLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            result.data?.data?.let { uri ->
                certFilePath = uri.toString()
                certFileName.text = uri.lastPathSegment ?: "certificate.pem"
                certFileName.setTextColor(resources.getColor(R.color.text_primary, null))
            }
        }
    }

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, arguments: Bundle?): View? {
        return inflater.inflate(R.layout.fragment_server_config, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, arguments)
        repository = ServersRepository(requireContext())
        prefs = requireContext().getSharedPreferences("secular_vpn_prefs", Context.MODE_PRIVATE)

        serverIndex = arguments?.getInt("serverIndex", -1) ?: -1

        // Find views
        fieldName = view.findViewById(R.id.field_server_name)
        fieldIp = view.findViewById(R.id.field_ip_address)
        fieldHostname = view.findViewById(R.id.field_hostname)
        fieldUsername = view.findViewById(R.id.field_username)
        fieldPassword = view.findViewById(R.id.field_password)
        fieldDns = view.findViewById(R.id.field_dns)
        fieldIpv6 = view.findViewById(R.id.field_ipv6)
        protocolSpinner = view.findViewById(R.id.field_protocol)
        certFileName = view.findViewById(R.id.cert_file_name)
        btnTogglePassword = view.findViewById(R.id.btn_toggle_password)
        btnDelete = view.findViewById(R.id.btn_delete)
        headerTitle = view.findViewById(R.id.config_header_title)

        // Setup protocol dropdown
        val protocols = arrayOf("HTTP/2", "QUIC")
        val spinnerAdapter = ArrayAdapter(requireContext(), android.R.layout.simple_spinner_dropdown_item, protocols)
        protocolSpinner.adapter = spinnerAdapter

        // Password toggle
        btnTogglePassword.setOnClickListener {
            passwordVisible = !passwordVisible
            if (passwordVisible) {
                fieldPassword.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
                btnTogglePassword.setImageResource(R.drawable.ic_eye_on)
            } else {
                fieldPassword.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                btnTogglePassword.setImageResource(R.drawable.ic_eye_off)
            }
            fieldPassword.setSelection(fieldPassword.text.length)
        }

        // Certificate upload
        view.findViewById<LinearLayout>(R.id.field_cert_upload).setOnClickListener {
            val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
                putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("application/octet-stream", "text/plain"))
            }
            certPickerLauncher.launch(intent)
        }

        // Save button
        view.findViewById<Button>(R.id.btn_save).setOnClickListener {
            saveConfig()
        }

        // Back button
        view.findViewById<ImageButton>(R.id.btn_back).setOnClickListener {
            findNavController().popBackStack()
        }

        // Delete button (only for existing servers)
        if (serverIndex >= 0) {
            btnDelete.visibility = View.VISIBLE
            btnDelete.setOnClickListener {
                showDeleteConfirmation()
            }
        }

        // Load existing data if editing
        loadServerData(view)
    }

    private fun loadServerData(view: View) {
        if (serverIndex < 0) {
            // New server mode — empty fields with defaults
            headerTitle.text = "New Server"
            fieldIpv6.isChecked = true
            protocolSpinner.setSelection(0)  // HTTP/2
            return
        }

        lifecycleScope.launch {
            val servers = repository.loadServers()
            if (serverIndex < servers.size) {
                val server = servers[serverIndex]
                existingServer = server

                fieldName.setText(server.name)
                fieldIp.setText(server.displayAddress)
                fieldHostname.setText(server.hostname)
                fieldUsername.setText(server.username)
                fieldPassword.setText(server.password)

                // DNS
                fieldDns.setText(server.dnsUpstreams.joinToString("\n"))

                // IPv6
                fieldIpv6.isChecked = server.hasIpv6

                // Protocol
                protocolSpinner.setSelection(
                    if (server.upstreamProtocol == "http3") 1 else 0
                )

                // Certificate
                if (server.certificate.isNotEmpty()) {
                    certFileName.text = server.certificate
                    certFilePath = server.certificate
                    certFileName.setTextColor(resources.getColor(R.color.text_primary, null))
                }

                if (view != null) {
                    headerTitle.text = server.name
                }
            }
        }
    }

    private fun saveConfig() {
        if (isSaving) return
        isSaving = true

        val name = fieldName.text.toString().trim()
        val ipAddress = fieldIp.text.toString().trim()
        val hostname = fieldHostname.text.toString().trim()
        val username = fieldUsername.text.toString().trim()
        val password = fieldPassword.text.toString()
        val ipv6 = fieldIpv6.isChecked
        val protocol = if (protocolSpinner.selectedItemPosition == 1) "http3" else "http2"
        val dnsServers = fieldDns.text.toString().lines()
            .map { it.trim() }
            .filter { it.isNotEmpty() }

        if (name.isEmpty()) {
            Toast.makeText(requireContext(), "Server name is required", Toast.LENGTH_SHORT).show()
            isSaving = false
            return
        }

        val profile = ServerProfile(
            name = name,
            hostname = hostname.ifEmpty { ipAddress },
            addresses = listOf(ipAddress).filter { it.isNotEmpty() },
            username = username,
            password = password,
            hasIpv6 = ipv6,
            upstreamProtocol = protocol,
            dnsUpstreams = dnsServers,
            certificate = certFilePath ?: ""
        )

        lifecycleScope.launch {
            try {
                if (serverIndex >= 0) {
                    repository.updateServer(serverIndex, profile)
                } else {
                    repository.addServer(profile)
                }
                // Persist selected server name
                prefs.edit().putString("selected_server_name", profile.name).apply()
                SecularVpnService.addLog("Config saved: ${profile.name}, navigating to server list")
                Toast.makeText(requireContext(), "Saved \u2713", Toast.LENGTH_SHORT).show()
            } catch (e: Exception) {
                SecularVpnService.addLog("Save error: ${e.message}")
            } finally {
                isSaving = false
            }
            // Navigate to server list to show the saved server
            navigateToServerList()
        }
    }

    private fun navigateToServerList() {
        // Pop back to the start destination (dashboard), then navigate to server list
        // This clears the add/config flow from the back stack
        findNavController().popBackStack(R.id.dashboardFragment, false)
        findNavController().navigate(R.id.action_dashboard_to_serverList)
    }

    private fun showDeleteConfirmation() {
        MaterialAlertDialogBuilder(requireContext())
            .setTitle("Delete Server")
            .setMessage("Are you sure you want to delete this server?")
            .setPositiveButton("Delete") { _, _ ->
                lifecycleScope.launch {
                    repository.deleteServer(serverIndex)
                    findNavController().popBackStack()
                }
            }
            .setNegativeButton("Cancel", null)
            .show()
    }
}
