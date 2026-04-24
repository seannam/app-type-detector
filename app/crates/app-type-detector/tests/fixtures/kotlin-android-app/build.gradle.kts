plugins {
    id("com.android.application")
    kotlin("android")
}

dependencies {
    implementation("androidx.compose.ui:ui:1.6.0")
    implementation("androidx.compose.material3:material3:1.2.0")
}

android { compileSdk = 34 }
