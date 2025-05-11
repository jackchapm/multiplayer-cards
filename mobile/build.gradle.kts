plugins {
    alias(libs.plugins.godotKotlinJvm)
    alias(libs.plugins.kotlin.serialization)
}

repositories {
    mavenCentral()
}

dependencies {
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.ktor.client.core)
    implementation(libs.ktor.client.cio)
    implementation(libs.ktor.client.contentNegotiation)
    implementation(libs.ktor.client.auth)
    implementation(libs.ktor.client.websockets)
    implementation(libs.ktor.serialization.kotlinx.json)
//    implementation(libs.ktor.client.logging)
    implementation(libs.logback.classic)
}

kotlin {
    jvmToolchain(23)
}

kotlin.sourceSets.main {
    kotlin.srcDirs("game")
}

godot {
    // the script registration which you'll attach to nodes are generated into this directory
    registrationFileBaseDir.set(projectDir.resolve("gdj").also { it.mkdirs() })
    isGodotCoroutinesEnabled.set(true)

    // NOTE: Make sure you read: https://godot-kotl.in/en/stable/user-guide/exporting/#android as not all jvm libraries are compatible with android!
    // TODO consider android 36
    isAndroidExportEnabled.set(false)
    d8ToolPath.set(File("${System.getenv("ANDROID_SDK_ROOT")}/build-tools/34.0.0/d8"))
    androidCompileSdkDir.set(File("${System.getenv("ANDROID_SDK_ROOT")}/platforms/android-34"))

    // NOTE: this is an advanced feature! Read: https://godot-kotl.in/en/stable/user-guide/advanced/graal-vm-native-image/
    // isGraalNativeImageExportEnabled.set(false)
    graalVmDirectory.set(File("${System.getenv("GRAALVM_HOME")}"))
    val graalDir = projectDir.resolve("graal")
    additionalGraalReflectionConfigurationFiles.set(arrayOf(graalDir.resolve("ktor-config.json").absolutePath))
    windowsDeveloperVCVarsPath.set(File("${System.getenv("VC_VARS_PATH")}"))
    isIOSExportEnabled.set(false)
}
