// secular-android/app/src/main/java/com/secular/vpn/data/ServersRepository.kt
// Server profiles JSON file storage

package com.secular.vpn.data

import android.content.Context
import com.google.gson.GsonBuilder
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

class ServersRepository(private val context: Context) {

    private val gson = GsonBuilder().setPrettyPrinting().create()
    private val serversFile: File
        get() = File(context.filesDir, "servers.json")

    @Suppress("UNCHECKED_CAST")
    suspend fun loadServers(): MutableList<ServerProfile> = withContext(Dispatchers.IO) {
        try {
            if (!serversFile.exists()) return@withContext mutableListOf()
            val json = serversFile.readText()
            val type = object : TypeToken<MutableList<ServerProfile>>() {}.type
            gson.fromJson<MutableList<ServerProfile>>(json, type) ?: mutableListOf()
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

    suspend fun addServer(server: ServerProfile) = withContext(Dispatchers.IO) {
        val servers = loadServers()
        servers.add(server)
        saveServers(servers)
    }

    suspend fun updateServer(index: Int, server: ServerProfile) = withContext(Dispatchers.IO) {
        val servers = loadServers()
        if (index in servers.indices) {
            servers[index] = server
            saveServers(servers)
        }
    }

    suspend fun deleteServer(index: Int) = withContext(Dispatchers.IO) {
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
