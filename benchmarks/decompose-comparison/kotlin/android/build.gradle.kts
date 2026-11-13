plugins {
    id("com.android.application")
}

android {
    namespace = "compono.compare"
    compileSdk = 36

    defaultConfig {
        applicationId = "compono.compare"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1"
    }

    sourceSets.getByName("main") {
        kotlin.directories.add("../kmp/src/commonMain/kotlin")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

dependencies {
    implementation("com.arkivanov.decompose:decompose:3.6.1")
}
