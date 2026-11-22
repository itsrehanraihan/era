import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

// Release signing (D-086). Values come from `keystore.properties` in the
// project root (keep it out of version control) or from environment
// variables. `era build android --release` sets the variables for you.
val keystoreProperties = Properties().apply {
    val f = rootProject.file("keystore.properties")
    if (f.isFile) f.inputStream().use { load(it) }
}
fun signingValue(property: String, env: String): String? =
    keystoreProperties.getProperty(property) ?: System.getenv(env)
val releaseStoreFile = signingValue("storeFile", "ERA_KEYSTORE")

android {
    namespace = "{{APP_ID}}"
    compileSdk = 35

    defaultConfig {
        applicationId = "{{APP_ID}}"
        minSdk = {{MIN_SDK}}
        targetSdk = 35
        versionCode = {{VERSION_CODE}}
        versionName = "{{VERSION_NAME}}"
    }

    signingConfigs {
        if (releaseStoreFile != null) {
            create("release") {
                storeFile = rootProject.file(releaseStoreFile)
                storePassword = signingValue("storePassword", "ERA_KEYSTORE_PASSWORD")
                keyAlias = signingValue("keyAlias", "ERA_KEY_ALIAS")
                keyPassword = signingValue("keyPassword", "ERA_KEY_PASSWORD") ?: storePassword
            }
        }
    }

    buildTypes {
        release {
            // R8 shrinks and optimises the code and removes unused resources.
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            signingConfig = signingConfigs.findByName("release")
        }
    }

    buildFeatures {
        compose = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.foundation)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.activity.compose)
}
