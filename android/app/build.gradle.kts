// Packages the pre-built Rust library into the Jondo APK. `android/build.sh`
// compiles jniLibs and runs the matching Gradle task.

import java.util.Properties

plugins {
    id("com.android.application")
}

// The release tag is the single source of truth: `-Pversion=1.2.3`.
val appVersion = (project.findProperty("version") as String?)
    ?.takeIf { it.isNotBlank() && it != "unspecified" }
    ?: "0.1.0"

fun versionCode(version: String): Int {
    val parts = version.split(".", "-", "+")
    fun part(index: Int) = parts.getOrNull(index)?.filter { it.isDigit() }?.toIntOrNull() ?: 0
    return maxOf(1, part(0) * 10000 + part(1) * 100 + part(2))
}

// The release key never lives in the repo: ANDROID_KEYSTORE_* in CI or a
// gitignored keystore.properties next to this build.
val keyProps = Properties()
val propsFile = rootProject.file("keystore.properties")
if (propsFile.exists()) propsFile.inputStream().use(keyProps::load)

fun signingValue(env: String, key: String): String? =
    System.getenv(env)?.takeIf { it.isNotBlank() } ?: keyProps.getProperty(key)?.takeIf { it.isNotBlank() }

val storePath = signingValue("ANDROID_KEYSTORE_FILE", "storeFile")
val storePasswordValue = signingValue("ANDROID_KEYSTORE_PASSWORD", "storePassword")
val keyAliasValue = signingValue("ANDROID_KEY_ALIAS", "keyAlias")
val keyPasswordValue = signingValue("ANDROID_KEY_PASSWORD", "keyPassword")
val hasReleaseKey = listOf(storePath, storePasswordValue, keyAliasValue, keyPasswordValue).all { it != null }

android {
    namespace = "com.jondo.jondo"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.jondo.jondo"
        minSdk = 26          // Vulkan 1.0 is mandatory from API 26+
        targetSdk = 35
        versionCode = versionCode(appVersion)
        versionName = appVersion

        // Tell NativeActivity which .so to load.
        // This must match the cdylib / example output name.
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }

        // Forward the library name to the manifest via a placeholder.
        manifestPlaceholders["nativeLibraryName"] = "jondo"
    }

    androidResources {
        // Stored, not deflated: an uncompressed asset is mmapped from the APK,
        // so the ten megabyte emoji font costs no resident pages.
        noCompress += listOf("ttf")
    }

    signingConfigs {
        if (hasReleaseKey) {
            create("release") {
                storeFile = rootProject.file(storePath!!)
                storePassword = storePasswordValue!!
                keyAlias = keyAliasValue!!
                keyPassword = keyPasswordValue!!
            }
        }
    }

    buildTypes {
        release {
            // A release without its own key is refused below, so this fallback
            // only keeps IDE sync and debug builds working.
            signingConfig = signingConfigs.findByName("release")
                ?: signingConfigs.getByName("debug")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
    }

    // We do NOT use CMake / ndk-build — the native library is compiled
    // externally via cargo-ndk and placed directly into jniLibs.
    //
    // Disable the built-in native build system so Gradle doesn't look for
    // a CMakeLists.txt or Android.mk.
    externalNativeBuild {
        // Intentionally left empty.
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    // Tell Gradle where the pre-built .so files live.
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        // Compressing the native libraries keeps the APK near 25 MB instead
        // of 43 MB. Android extracts them once at install; the extracted copy
        // is what gets mapped, so playback is unaffected.
        jniLibs {
            useLegacyPackaging = true
            keepDebugSymbols += listOf(
                "*/arm64-v8a/libjondo.so",
                "*/armeabi-v7a/libjondo.so",
                "*/x86_64/libjondo.so",
                "*/x86/libjondo.so"
            )
        }
    }

    // Lint configuration — relaxed for an example project.
    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

gradle.taskGraph.whenReady {
    if (allTasks.any { it.name.contains("Release") } && !hasReleaseKey) {
        throw GradleException(
            "Android release signing is not configured: set ANDROID_KEYSTORE_FILE, " +
                "ANDROID_KEYSTORE_PASSWORD, ANDROID_KEY_ALIAS and ANDROID_KEY_PASSWORD, " +
                "or add a gitignored android/keystore.properties."
        )
    }
}

dependencies {
    // AndroidX core for NotificationCompat (used by GpuiNotifications)
    implementation("androidx.core:core:1.12.0")
    // AndroidX SplashScreen compat (used by GpuiActivity to hold splash until native init)
    implementation("androidx.core:core-splashscreen:1.0.1")
    // AndroidX Media for MediaSessionCompat (used by GpuiMediaSession for system controls)
    implementation("androidx.media:media:1.7.1")
}
