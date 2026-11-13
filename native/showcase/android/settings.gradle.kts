pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "showcase-android"
include(":app")
include(":compono")
project(":compono").projectDir = file("../../android/compono")
