import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    alias(libs.plugins.godotKotlinJvm)
    alias(libs.plugins.kotlin.serialization)
}

repositories {
    mavenCentral()
}

val debugImplementation by configurations.creating {
    extendsFrom(configurations.implementation.get())
}

dependencies {
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.ktor.client.core)
    implementation(libs.ktor.client.cio)
    implementation(libs.ktor.client.contentNegotiation)
    implementation(libs.ktor.client.auth)
    implementation(libs.ktor.client.websockets)
    implementation(libs.ktor.serialization.kotlinx.json)
//    debugImplementation(libs.ktor.client.logging)
//    debugImplementation(libs.logback.classic)
}

kotlin {
    jvmToolchain(23)
}

kotlin.sourceSets.main {
    kotlin.srcDirs("game")
}

val isDebugBuild = project.gradle.startParameter.taskNames.any { it.contains("debugBuild") }

godot {
    // the script registration which you'll attach to nodes are generated into this directory
    registrationFileBaseDir.set(projectDir.resolve("gdj").also { it.mkdirs() })
    isGodotCoroutinesEnabled.set(true)

    // NOTE: Make sure you read: https://godot-kotl.in/en/stable/user-guide/exporting/#android as not all jvm libraries are compatible with android!
    // TODO consider android 36
    androidMinApi.set(30)
    d8ToolPath.set(File("${System.getenv("ANDROID_SDK")}/build-tools/36.0.0/d8"))
    androidCompileSdkDir.set(File("${System.getenv("ANDROID_SDK")}/platforms/android-36"))

    // NOTE: this is an advanced feature! Read: https://godot-kotl.in/en/stable/user-guide/advanced/graal-vm-native-image/
    // isGraalNativeImageExportEnabled.set(false)
    graalVmDirectory.set(File("${System.getenv("GRAALVM_HOME")}"))
    val graalDir = projectDir.resolve("graal")
    additionalGraalReflectionConfigurationFiles.set(arrayOf(graalDir.resolve("ktor-config.json").absolutePath))
    windowsDeveloperVCVarsPath.set(File("${System.getenv("VC_VARS_PATH")}"))

    isIOSExportEnabled.set(!isDebugBuild)
    isAndroidExportEnabled.set(!isDebugBuild)
}

val compileKotlin: KotlinCompile by tasks
compileKotlin.compilerOptions {
    freeCompilerArgs.set(listOf("-Xwhen-guards"))
}

tasks.register("debugBuild") {
    group = "build"
    description = "Build the project targetting jvm for debugging rather than a full native build"
    dependsOn("build")
}
