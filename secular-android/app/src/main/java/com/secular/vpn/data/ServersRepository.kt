// secular-android/app/src/main/java/com/secular/vpn/data/ServersRepository.kt
// Server profiles JSON file storage

package com.secular.vpn.data

import android.content.Context
import com.google.gson.GsonBuilder
import com.google.gson.reflect.TypeToken
import com.secular.vpn.SecularVpnService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File

class ServersRepository(private val context: Context) {

    private val gson = GsonBuilder().setPrettyPrinting().create()
    private val serversFile: File
        get() = File(context.filesDir, "servers.json")

    // Mutex to prevent concurrent writes causing duplicates
    private val writeMutex = Mutex()

    @Suppress("UNCHECKED_CAST")
    suspend fun loadServers(): MutableList<ServerProfile> = withContext(Dispatchers.IO) {
        try {
            if (!serversFile.exists()) {
                SecularVpnService.addLog("Repo: loadServers() — no file, returning empty")
                return@withContext mutableListOf()
            }
            val json = serversFile.readText()
            val type = object : TypeToken<MutableList<ServerProfile>>() {}.type
            val loaded = gson.fromJson<MutableList<ServerProfile>>(json, type) ?: mutableListOf()
            SecularVpnService.addLog("Repo: loadServers() — raw count=${loaded.size}, names=${loaded.map { it.name }}")
            // Filter out stale entries with no name and no address
            val originalSize = loaded.size
            loaded.removeAll { it.name.isEmpty() && it.hostname.isEmpty() && it.addresses.isEmpty() }
            // Deduplicate by name (keep first occurrence)
            val seen = mutableSetOf<String>()
            loaded.removeAll { server ->
                if (server.name.isEmpty()) false
                else if (seen.contains(server.name)) true
                else { seen.add(server.name); false }
            }
            if (loaded.size != originalSize) {
                SecularVpnService.addLog("Repo: loadServers() — cleaned ${originalSize} -> ${loaded.size}, saving")
                saveServers(loaded)
            }
            loaded
        } catch (e: Exception) {
            SecularVpnService.addLog("Repo: loadServers() — ERROR: ${e.javaClass.simpleName}: ${e.message}")
            mutableListOf()
        }
    }

    suspend fun saveServers(servers: List<ServerProfile>) = withContext(Dispatchers.IO) {
        try {
            val json = gson.toJson(servers)
            SecularVpnService.addLog("Repo: saveServers() — writing ${servers.size} servers: ${json.length} chars")
            serversFile.writeText(json)
        } catch (e: Exception) {
            SecularVpnService.addLog("Repo: saveServers() — ERROR: ${e.message}")
        }
    }

    suspend fun addServer(server: ServerProfile): Int = writeMutex.withLock {
        SecularVpnService.addLog("Repo: addServer(name=${server.name}, addr=${server.addresses}) START")
        val servers = loadServers()
        SecularVpnService.addLog("Repo: addServer — existing=${servers.size}, names=${servers.map { it.name }}")
        // Dedup: skip if same name exists
        val existsIdx = servers.indexOfFirst { it.name == server.name }
        if (existsIdx >= 0) {
            SecularVpnService.addLog("Repo: addServer — DUPLICATE name=${server.name} at idx=$existsIdx, UPDATING")
            servers[existsIdx] = server
            saveServers(servers)
            return existsIdx
        }
        servers.add(server)
        SecularVpnService.addLog("Repo: addServer — NEW at idx=${servers.size - 1}, saving")
        saveServers(servers)
        servers.size - 1
    }

    suspend fun updateServer(index: Int, server: ServerProfile) = writeMutex.withLock {
        SecularVpnService.addLog("Repo: updateServer(idx=$index, name=${server.name})")
        val servers = loadServers()
        if (index in servers.indices) {
            servers[index] = server
            saveServers(servers)
        } else {
            SecularVpnService.addLog("Repo: updateServer — INVALID index $index, size=${servers.size}")
        }
    }

    suspend fun deleteServer(index: Int) = writeMutex.withLock {
        val servers = loadServers()
        if (index in servers.indices) {
            servers.removeAt(index)
            saveServers(servers)
        }
    }

    suspend fun getServer(index: Int): ServerProfile? = withContext(Dispatchers.IO) {
        val servers = loadServers()
        if (index in servers.indices) servers[index] else null
    }
}
