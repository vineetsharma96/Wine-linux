plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

val rootfsReleaseKey = providers.environmentVariable("ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX").orElse("")

android {
    namespace = "com.androidpc.environment"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.androidpc.environment"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0-alpha.0"
        val publicKey = rootfsReleaseKey.get().takeIf { it.matches(Regex("[0-9a-fA-F]{64}")) }.orEmpty()
        buildConfigField("String", "ROOTFS_PUBLIC_KEY_HEX", "\"" + publicKey + "\"")
        buildConfigField("String", "BOX64_VERSION", "\"0.4.4\"")
        buildConfigField("String", "WINE_VERSION", "\"11.0\"")
        buildConfigField("String", "TURNIP_VERSION", "\"26.3.1\"")
        buildConfigField("String", "VIRGL_VERSION", "\"1.3.0\"")
        buildConfigField("String", "DXVK_VERSION", "\"2.7.1\"")
        buildConfigField("String", "VKD3D_VERSION", "\"3.1\"")
        buildConfigField("String", "QEMU_VERSION", "\"11.1.3\"")
        buildConfigField("String", "EDK2_VERSION", "\"edk2-stable202608\"")
        buildConfigField("String", "LIBSLIRP_VERSION", "\"4.9.7\"")

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            abiFilters.add("arm64-v8a")
        }
    }

    signingConfigs {
        create("debugConfig") {
            storeFile = file("${rootDir}/debug.keystore")
            storePassword = "android"
            keyAlias = "androiddebugkey"
            keyPassword = "android"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isDebuggable = true
            signingConfig = signingConfigs.getByName("debugConfig")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
        if (rootfsReleaseKey.get().isNotBlank()) {
            getByName("debug").assets.srcDir("src/release/assets")
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

tasks.register("verifyReleaseRootfsInputs") {
    doLast {
        check(rootfsReleaseKey.get().matches(Regex("[0-9a-fA-F]{64}"))) {
            "Set ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX to the trusted release Ed25519 public key"
        }
        val rootfsAssets = layout.projectDirectory.dir("src/release/assets/rootfs")
        check(rootfsAssets.file("catalog.json").asFile.isFile &&
            rootfsAssets.file("catalog.sig").asFile.length() == 64L) {
            "Stage the signed release catalog in src/release/assets/rootfs"
        }
    }
}

tasks.matching { it.name == "preReleaseBuild" }.configureEach {
    dependsOn("verifyReleaseRootfsInputs")
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.ui.tooling.preview)
    implementation(libs.androidx.material3)
    implementation(libs.androidx.material.icons.extended)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.google.gson)

    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    debugImplementation(libs.androidx.ui.tooling)
}
