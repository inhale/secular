# secular-android/app/proguard-rules.pro
# Secular Android — ProGuard rules

# Keep VPN service
-keep class com.secular.vpn.SecularVpnService { *; }
-keep class com.secular.vpn.MainActivity { *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}

# UniFFI / JNI
-keep class com.secular.vpn.core.** { *; }
