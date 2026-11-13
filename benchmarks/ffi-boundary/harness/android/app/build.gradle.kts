plugins {
    id("com.android.application")
}

android {
    namespace = "compono.bench"
    compileSdk = 36

    defaultConfig {
        applicationId = "compono.bench"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1"
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            isDebuggable = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

dependencies {
    if (providers.gradleProperty("backend").orNull == "uni") {
        implementation("net.java.dev.jna:jna:5.18.1@aar")
    }
}
