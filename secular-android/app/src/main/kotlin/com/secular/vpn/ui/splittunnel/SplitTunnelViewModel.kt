package com.secular.vpn.ui.splittunnel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.secular.vpn.data.splittunnel.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

class SplitTunnelViewModel(
    private val repository: SplitTunnelRepository
) : ViewModel() {
    
    private val _settings = MutableStateFlow(repository.getSettings())
    val settings: StateFlow<SplitTunnelSettings> = _settings.asStateFlow()
    
    private val _installedApps = MutableStateFlow<List<AppInfo>>(emptyList())
    val installedApps: StateFlow<List<AppInfo>> = _installedApps.asStateFlow()
    
    private val _searchQuery = MutableStateFlow("")
    val searchQuery: StateFlow<String> = _searchQuery.asStateFlow()
    
    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()
    
    val filteredApps: StateFlow<List<AppInfo>> = combine(
        installedApps,
        searchQuery,
        settings
    ) { apps, query, settings ->
        apps.filter { app ->
            app.appName.contains(query, ignoreCase = true) ||
            app.packageName.contains(query, ignoreCase = true)
        }.map { app ->
            app.copy(
                isSelected = when (settings.mode) {
                    SplitTunnelMode.EXCLUDE_SELECTED -> app.packageName in settings.excludedApps
                    SplitTunnelMode.ONLY_SELECTED -> app.packageName in settings.allowedApps
                    else -> false
                }
            )
        }
    }.stateIn(viewModelScope, SharingStarted.Lazily, emptyList())
    
    val selectedCount: StateFlow<Int> = combine(
        filteredApps,
        settings
    ) { apps, settings ->
        when (settings.mode) {
            SplitTunnelMode.EXCLUDE_SELECTED -> settings.excludedApps.size
            SplitTunnelMode.ONLY_SELECTED -> settings.allowedApps.size
            else -> 0
        }
    }.stateIn(viewModelScope, SharingStarted.Lazily, 0)
    
    init {
        loadInstalledApps()
    }
    
    fun loadInstalledApps() {
        viewModelScope.launch {
            _isLoading.value = true
            _installedApps.value = repository.getInstalledApps()
            _isLoading.value = false
        }
    }
    
    fun setMode(mode: SplitTunnelMode) {
        viewModelScope.launch {
            val updated = _settings.value.copy(mode = mode)
            repository.saveSettings(updated)
            _settings.value = updated
        }
    }
    
    fun toggleApp(packageName: String) {
        viewModelScope.launch {
            val current = _settings.value
            val updated = when (current.mode) {
                SplitTunnelMode.EXCLUDE_SELECTED -> {
                    if (packageName in current.excludedApps) {
                        current.copy(excludedApps = current.excludedApps - packageName)
                    } else {
                        current.copy(excludedApps = current.excludedApps + packageName)
                    }
                }
                SplitTunnelMode.ONLY_SELECTED -> {
                    if (packageName in current.allowedApps) {
                        current.copy(allowedApps = current.allowedApps - packageName)
                    } else {
                        current.copy(allowedApps = current.allowedApps + packageName)
                    }
                }
                else -> current
            }
            repository.saveSettings(updated)
            _settings.value = updated
        }
    }
    
    fun setSearchQuery(query: String) {
        _searchQuery.value = query
    }
    
    fun clearAllSelections() {
        viewModelScope.launch {
            val updated = _settings.value.copy(
                excludedApps = emptySet(),
                allowedApps = emptySet()
            )
            repository.saveSettings(updated)
            _settings.value = updated
        }
    }
}
