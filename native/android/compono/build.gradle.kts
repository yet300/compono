plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "compono.android"
    compileSdk = 37

    defaultConfig {
        minSdk = 26
    }

    buildFeatures {
        compose = true
    }

    testOptions {
        unitTests.isReturnDefaultValues = true
    }
}

dependencies {
    api("androidx.compose.runtime:runtime:1.9.0")
    api("androidx.compose.foundation:foundation:1.9.0")
    api("androidx.compose.animation:animation:1.9.0")
    implementation("androidx.compose.ui:ui-graphics:1.9.0")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.0")
    testImplementation("junit:junit:4.13.2")
}

// The motion tests compare with the Rust crate's golden table, so a new table reruns them.
tasks.withType<Test>().configureEach {
    inputs.file("../../../crates/compono-motion/golden/motion.csv")
}
