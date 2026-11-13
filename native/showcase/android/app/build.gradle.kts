plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "compono.showcase"
    compileSdk = 37

    defaultConfig {
        applicationId = "compono.showcase"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1"
    }

    buildFeatures {
        compose = true
    }

    sourceSets.getByName("main") {
        // Built by `boltffi pack android --release` in ../ffi.
        kotlin.directories.add("../../ffi/dist/android/kotlin")
        jniLibs.srcDir("../../ffi/dist/android/jniLibs")
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

dependencies {
    implementation(project(":compono"))
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.compose.material3:material3:1.4.0")
    implementation("androidx.compose.material3:material3-adaptive-navigation-suite:1.4.0")
    implementation("androidx.compose.material3.adaptive:adaptive:1.3.0")
    implementation("androidx.compose.material3.adaptive:adaptive-layout:1.3.0")
    implementation("androidx.compose.material:material-icons-core:1.7.8")
}
