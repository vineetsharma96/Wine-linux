plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

val rootfsReleaseKey = providers.environmentVariable("ANDROIDPC_ROOTFS_PUBLIC_KEY_HEX").orElse("")

android {
    namespace = "com.androidpc.environment"
    compileSdk = 35
    ndkVersion = "30.0.16248370"

    defaultConfig {
        applicationId = "com.androidpc.environment"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0-alpha.0"
        val publicKey = rootfsReleaseKey.get().takeIf { it.matches(Regex("[0-9a-fA-F]{64}")) }.orEmpty()
        buildConfigField("String", "ROOTFS_PUBLIC_KEY_HEX", "\"" + publicKey + "\"")

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            abiFilters.add("arm64-v8a")
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
            applicationIdSuffix = ".debug"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
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
