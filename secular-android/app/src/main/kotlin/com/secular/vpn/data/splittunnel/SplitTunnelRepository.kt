package com.secular.vpn.data.splittunnel

import android.content.Context
import android.content.SharedPreferences
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class SplitTunnelRepository(private val context: Context) {
    
    private val prefs: SharedPreferences = context.getSharedPreferences(
        "split_tunnel_settings",
        Context.MODE_PRIVATE
    )
    
    private val json = Json { prettyPrint = true }
    
    fun getSettings(): SplitTunnelSettings {
        val modeStr = prefs.getString("mode", SplitTunnelMode.ALL_THROUGH_VPN.name)
        val mode = SplitTunnelMode.valueOf(modeStr ?: SplitTunnelMode.ALL_THROUGH_VPN.name)
        
        val excludedAppsStr = prefs.getStringSet("excluded_apps", emptySet()) ?: emptySet()
        val allowedAppsStr = prefs.getStringSet("allowed_apps", emptySet()) ?: emptySet()
        
        return SplitTunnelSettings(
            mode = mode,
            excludedApps = excludedAppsStr,
            allowedApps = allowedAppsStr
        )
    }
    
    fun saveSettings(settings: SplitTunnelSettings) {
        prefs.edit().apply {
            putString("mode", settings.mode.name)
            putStringSet("excluded_apps", settings.excludedApps)
            putStringSet("allowed_apps", settings.allowedApps)
            apply()
        }
    }
    
    suspend fun getInstalledApps(): List<AppInfo> = withContext(Dispatchers.IO) {
        val pm = context.packageManager
        val settings = getSettings()
        
        pm.getInstalledApplications(PackageManager.GET_META_DATA)
            .filter { appInfo ->
                // Only user-installed apps (not system apps)
                (appInfo.flags and ApplicationInfo.FLAG_SYSTEM) == 0
            }
            .map { appInfo ->
                val packageName = appInfo.packageName
                val isSelected = when (settings.mode) {
                    SplitTunnelMode.EXCLUDE_SELECTED -> packageName in settings.excludedApps
                    SplitTunnelMode.ONLY_SELECTED -> packageName in settings.allowedApps
                    else -> false
                }
                
                AppInfo(
                    packageName = packageName,
                    appName = appInfo.loadLabel(pm).toString(),
                    isSystemApp = false,
                    isSelected = isSelected
                )
            }
            .sortedBy { it.appName }
    }
}
