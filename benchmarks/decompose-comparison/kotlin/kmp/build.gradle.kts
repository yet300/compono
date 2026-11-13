plugins {
    id("org.jetbrains.kotlin.multiplatform")
}

kotlin {
    jvm {
        binaries {
            executable {
                mainClass.set("compono.compare.MainKt")
            }
        }
    }

    macosArm64 {
        binaries.executable {
            entryPoint = "compono.compare.main"
        }
    }

    iosSimulatorArm64 {
        binaries.executable {
            entryPoint = "compono.compare.main"
        }
    }

    sourceSets {
        commonMain.dependencies {
            implementation("com.arkivanov.decompose:decompose:3.6.1")
        }
    }
}
