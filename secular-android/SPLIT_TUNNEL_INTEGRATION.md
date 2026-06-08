# Android Split Tunneling - Navigation Integration

To integrate the SplitTunnelFragment into your navigation, add this to your MainActivity or navigation graph:

## If using FragmentManager (traditional):

```kotlin
// In MainActivity.kt or wherever you handle navigation
fun openSplitTunneling() {
    supportFragmentManager.beginTransaction()
        .replace(R.id.fragment_container, SplitTunnelFragment())
        .addToBackStack(null)
        .commit()
}
```

## If using Navigation Component:

Add to `res/navigation/nav_graph.xml`:

```xml
<fragment
    android:id="@+id/splitTunnelFragment"
    android:name="com.secular.vpn.ui.splittunnel.SplitTunnelFragment"
    android:label="Split Tunneling" />
```

## Add menu item to settings:

In your settings fragment or dashboard:

```kotlin
// Add a button/menu item
settingsButton.setOnClickListener {
    findNavController().navigate(R.id.splitTunnelFragment)
}
```

## Dependencies needed:

Add to `app/build.gradle.kts`:

```kotlin
dependencies {
    // Jetpack Compose (if not already added)
    implementation("androidx.compose.ui:ui:1.5.4")
    implementation("androidx.compose.material3:material3:1.1.2")
    implementation("androidx.activity:activity-compose:1.8.0")
    
    // Lifecycle
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.6.2")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.6.2")
    
    // Kotlin Serialization
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.0")
    
    // Coil for loading app icons
    implementation("io.coil-kt:coil-compose:2.5.0")
}
```

Enable Kotlin serialization in `app/build.gradle.kts`:

```kotlin
plugins {
    kotlin("plugin.serialization") version "1.9.20"
}
```

## Testing:

1. Build and run the app
2. Navigate to Split Tunneling screen
3. Select a mode (Exclude/Only)
4. Toggle some apps
5. Connect to VPN
6. Check logs for split tunnel messages
7. Verify excluded apps bypass VPN (e.g., curl ifconfig.me in excluded browser)
