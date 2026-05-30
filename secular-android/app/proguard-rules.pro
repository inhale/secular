# secular-android/app/proguard-rules.pro
# Secular Android — ProGuard rules

# Keep VPN service
-keep class com.secular.vpn.SecularVpnService { *; }
-keep class com.secular.vpn.MainActivity { *; }

# Keep data classes (JSON serialization)
-keep class com.secular.vpn.data.** { *; }

# Keep UI fragments
-keep class com.secular.vpn.ui.** { *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}

# UniFFI / JNI
-keep class com.secular.vpn.core.** { *; }

# Gson
-keepattributes Signature
-keepattributes *Annotation*
-keepclassmembers class * {
    @com.google.gson.annotations.SerializedName <fields>;
}
