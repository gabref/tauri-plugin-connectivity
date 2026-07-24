plugins {
    id("org.jetbrains.kotlin.jvm")
}

// Pure connectivity mapping, isolated from the plugin module — that module
// depends on the Android framework and the Tauri Android API, neither of which
// is available when running unit tests on the JVM.
kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}

tasks.withType<Test>().configureEach {
    testLogging {
        events("passed", "skipped", "failed")
    }
}
