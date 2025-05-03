import korlibs.korge.gradle.*

plugins {
	alias(libs.plugins.korge)
}

korge {
	id = "com.jackchap.multiplayercards"
	name = "Multiplayer Cards"
    preferredIphoneSimulatorVersion = 16
	targetIos()
//	targetAndroid()

	serializationJson()
}


dependencies {
    add("commonMainApi", project(":deps"))
}

