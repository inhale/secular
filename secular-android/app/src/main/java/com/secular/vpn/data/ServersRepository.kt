// secular-android/app/src/main/java/com/secular/vpn/data/ServersRepository.kt
// Server profiles JSON file storage

package com.secular.vpn.data

import android.content.Context
import com.google.gson.GsonBuilder
import com.google.gson.reflect.TypeToken
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
            if (!serversFile.exists()) return@withContext mutableListOf()
            val json = serversFile.readText()
            val type = object : TypeToken<MutableList<ServerProfile>>() {}.type
            val loaded = gson.fromJson<MutableList<ServerProfile>>(json, type) ?: mutableListOf()
            // Filter out stale entries with no name and no address
            val originalSize = loaded.size
            loaded.removeAll { it.name.isEmpty() && it.hostname.isEmpty() && it.addresses.isEmpty() }
            // Deduplicate by name (keep first occurrence, update with latest data)
            val seen = mutableSetOf<String>()
            loaded.removeAll { server ->
                if (server.name.isEmpty()) false
                else if (seen.contains(server.name)) true
                else { seen.add(server.name); false }
            }
            if (loaded.size != originalSize) saveServers(loaded)
            loaded
        } catch (e: Exception) {
            mutableListOf()
        }
    }

    suspend fun saveServers(servers: List<ServerProfile>) = withContext(Dispatchers.IO) {
        try {
            serversFile.writeText(gson.toJson(servers))
        } catch (e: Exception) {
            // silent fail
        }
    }

    suspend fun addServer(server: ServerProfile): Int = writeMutex.withLock {
        val servers = loadServers()
        // Dedup: skip if same name OR same hostname+address
        val existsIdx = servers.indexOfFirst {
            it.name == server.name ||
            (it.hostname == server.hostname && it.hostname.isNotEmpty() &&
             it.addresses == server.addresses && server.addresses.isNotEmpty())
        }
        if (existsIdx >= 0) {
            // Update existing entry instead of duplicating
            servers[existsIdx] = server
            saveServers(servers)
            return existsIdx
        }
        servers.add(server)
        saveServers(servers)
        servers.size - 1  // return index of newly added server
    }

    suspend fun updateServer(index: Int, server: ServerProfile) = writeMutex.withLock {
        val servers = loadServers()
        if (index in servers.indices) {
            servers[index] = server
            saveServers(servers)
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
