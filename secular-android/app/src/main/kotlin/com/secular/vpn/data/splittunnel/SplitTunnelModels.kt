package com.secular.vpn.data.splittunnel

import kotlinx.serialization.Serializable

@Serializable
enum class SplitTunnelMode {
    ALL_THROUGH_VPN,      // Route everything through VPN
    EXCLUDE_SELECTED,      // Exclude selected apps (they go direct)
    ONLY_SELECTED          // Only route selected apps through VPN
}

data class AppInfo(
    val packageName: String,
    val appName: String,
    val isSystemApp: Boolean,
    val isSelected: Boolean = false
)

@Serializable
data class SplitTunnelSettings(
    val mode: SplitTunnelMode = SplitTunnelMode.ALL_THROUGH_VPN,
    val excludedApps: Set<String> = emptySet(),
    val allowedApps: Set<String> = emptySet()
)
