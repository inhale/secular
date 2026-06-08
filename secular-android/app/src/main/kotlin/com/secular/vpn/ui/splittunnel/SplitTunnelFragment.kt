package com.secular.vpn.ui.splittunnel

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.fragment.app.Fragment
import androidx.lifecycle.ViewModelProvider
import coil.compose.rememberAsyncImagePainter
import com.secular.vpn.SecularApplication
import com.secular.vpn.data.splittunnel.*

class SplitTunnelFragment : Fragment() {
    
    private lateinit fun viewModel: SplitTunnelViewModel
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        val app = requireActivity().application as SecularApplication
        val repository = SplitTunnelRepository(app.applicationContext)
        viewModel = ViewModelProvider(
            this,
            SplitTunnelViewModelFactory(repository)
        )[SplitTunnelViewModel::class.java]
    }
    
    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View {
        return ComposeView(requireContext()).apply {
            setContent {
                MaterialTheme {
                    SplitTunnelScreen(
                        viewModel = viewModel,
                        onBack = { requireActivity().onBackPressed() }
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SplitTunnelScreen(
    viewModel: SplitTunnelViewModel,
    onBack: () -> Unit
) {
    val settings by viewModel.settings.collectAsState()
    val filteredApps by viewModel.filteredApps.collectAsState()
    val searchQuery by viewModel.searchQuery.collectAsState()
    val selectedCount by viewModel.selectedCount.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()
    
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Split Tunneling") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Mode Selection Card
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(
                        text = "Route Mode",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold
                    )
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    RadioButtonRow(
                        label = "Route all apps through VPN",
                        selected = settings.mode == SplitTunnelMode.ALL_THROUGH_VPN,
                        onClick = { viewModel.setMode(SplitTunnelMode.ALL_THROUGH_VPN) }
                    )
                    
                    RadioButtonRow(
                        label = "Exclude selected apps",
                        subtitle = "Selected apps bypass VPN",
                        selected = settings.mode == SplitTunnelMode.EXCLUDE_SELECTED,
                        onClick = { viewModel.setMode(SplitTunnelMode.EXCLUDE_SELECTED) }
                    )
                    
                    RadioButtonRow(
                        label = "Only route selected apps",
                        subtitle = "Only selected apps go through VPN",
                        selected = settings.mode == SplitTunnelMode.ONLY_SELECTED,
                        onClick = { viewModel.setMode(SplitTunnelMode.ONLY_SELECTED) }
                    )
                }
            }
            
            // Search and App List (only show if not ALL_THROUGH_VPN)
            if (settings.mode != SplitTunnelMode.ALL_THROUGH_VPN) {
                // Search Bar
                OutlinedTextField(
                    value = searchQuery,
                    onValueChange = { viewModel.setSearchQuery(it) },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp),
                    placeholder = { Text("🔍 Search apps...") },
                    leadingIcon = { Icon(Icons.Default.Search, "Search") },
                    trailingIcon = {
                        if (searchQuery.isNotEmpty()) {
                            IconButton(onClick = { viewModel.setSearchQuery("") }) {
                                Icon(Icons.Default.Clear, "Clear")
                            }
                        }
                    },
                    singleLine = true
                )
                
                Spacer(modifier = Modifier.height(8.dp))
                
                // Selected Count + Clear All
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "$selectedCount app${if (selectedCount != 1) "s" else ""} selected",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.secondary
                    )
                    
                    if (selectedCount > 0) {
                        TextButton(onClick = { viewModel.clearAllSelections() }) {
                            Text("Clear All")
                        }
                    }
                }
                
                Spacer(modifier = Modifier.height(8.dp))
                
                // App List
                if (isLoading) {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center
                    ) {
                        CircularProgressIndicator()
                    }
                } else {
                    LazyColumn(
                        modifier = Modifier.fillMaxSize()
                    ) {
                        items(filteredApps) { app ->
                            AppListItem(
                                app = app,
                                onToggle = { viewModel.toggleApp(app.packageName) }
                            )
                        }
                    }
                }
            } else {
                // ALL_THROUGH_VPN mode - show centered message
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "All apps are routed through VPN",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
}

@Composable
fun AppListItem(
    app: AppInfo,
    onToggle: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onToggle() }
            .padding(16.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // App icon placeholder (you'll need to load actual icon)
        Surface(
            modifier = Modifier.size(48.dp),
            shape = MaterialTheme.shapes.small
        ) {
            // TODO: Load actual app icon using rememberAsyncImagePainter
        }
        
        Spacer(modifier = Modifier.width(16.dp))
        
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = app.appName,
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium
            )
            Text(
                text = app.packageName,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        
        Switch(
            checked = app.isSelected,
            onCheckedChange = { onToggle() }
        )
    }
}

@Composable
fun RadioButtonRow(
    label: String,
    subtitle: String? = null,
    selected: Boolean,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() }
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        RadioButton(
            selected = selected,
            onClick = onClick
        )
        
        Spacer(modifier = Modifier.width(8.dp))
        
        Column {
            Text(
                text = label,
                style = MaterialTheme.typography.bodyLarge
            )
            if (subtitle != null) {
                Text(
                    text = subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

class SplitTunnelViewModelFactory(
    private val repository: SplitTunnelRepository
) : ViewModelProvider.Factory {
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        if (modelClass.isAssignableFrom(SplitTunnelViewModel::class.java)) {
            @Suppress("UNCHECKED_CAST")
            return SplitTunnelViewModel(repository) as T
        }
        throw IllegalArgumentException("Unknown ViewModel class")
    }
}
