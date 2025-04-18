package com.jackchap.multiplayercards

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform