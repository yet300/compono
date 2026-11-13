plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.vanniktech.maven.publish") version "0.37.0"
}

// The library has the version of the Rust crates it was tested with: the
// workspace version in the root Cargo.toml.
val componoVersion: String = file("../../../Cargo.toml").readLines()
    .first { it.startsWith("version = ") }
    .substringAfter('"').substringBefore('"')

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

// `./gradlew :compono:publishToMavenCentral` uploads a signed deployment to the
// Central Portal, where it is released by hand. The credentials and the signing key
// come from `mavenCentralUsername`, `mavenCentralPassword` and `signingInMemoryKey*`
// in `~/.gradle/gradle.properties`.
mavenPublishing {
    publishToMavenCentral()
    signAllPublications()
    coordinates("io.github.yet300", "compono", componoVersion)
    pom {
        name.set("Compono")
        description.set("Jetpack Compose views for Compono component trees: routers, lifecycle, Back gestures and transitions.")
        inceptionYear.set("2026")
        url.set("https://github.com/yet300/compono")
        licenses {
            license {
                name.set("Apache-2.0")
                url.set("https://www.apache.org/licenses/LICENSE-2.0.txt")
                distribution.set("repo")
            }
        }
        developers {
            developer {
                id.set("yet300")
                name.set("yet300")
                url.set("https://github.com/yet300")
            }
        }
        scm {
            url.set("https://github.com/yet300/compono")
            connection.set("scm:git:https://github.com/yet300/compono.git")
            developerConnection.set("scm:git:ssh://git@github.com/yet300/compono.git")
        }
    }
}
