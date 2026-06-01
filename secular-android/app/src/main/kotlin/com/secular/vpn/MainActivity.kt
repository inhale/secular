// secular-android/app/src/main/kotlin/com/secular/vpn/MainActivity.kt
// Secular Android — Main Activity (hosts navigation graph)

package com.secular.vpn

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.navigation.fragment.NavHostFragment

class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        // Find the NavHostFragment and set up navigation
        val navHostFragment = supportFragmentManager
            .findFragmentById(R.id.nav_host_fragment) as NavHostFragment
        @Suppress("UNUSED_VARIABLE")
        val navController = navHostFragment.navController
    }
}
