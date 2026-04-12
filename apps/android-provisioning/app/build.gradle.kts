import java.time.Instant

plugins {
    id("com.android.application")
}

val localBuildRoot = System.getenv("LUMELO_ANDROID_BUILD_ROOT") ?: "/tmp/lumelo-android-build"
val buildTimeUtc = Instant.now().toString()

fun shortSha(value: String?): String? {
    if (value.isNullOrBlank()) {
        return null
    }
    return value.trim().take(8)
}

val gitShaShort = shortSha(System.getenv("LUMELO_GIT_SHA_SHORT"))
    ?: shortSha(System.getenv("GIT_COMMIT"))
    ?: shortSha(System.getenv("GITHUB_SHA"))
    ?: "nogit"

layout.buildDirectory.set(file("$localBuildRoot/app"))

android {
    namespace = "com.lumelo.provisioning"
    compileSdk {
        version = release(36) {
            minorApiLevel = 1
        }
    }

    defaultConfig {
        applicationId = "com.lumelo.provisioning"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        buildConfigField("String", "BUILD_TIME_UTC", "\"$buildTimeUtc\"")
        buildConfigField("String", "GIT_SHA_SHORT", "\"$gitShaShort\"")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        buildConfig = true
    }
}
