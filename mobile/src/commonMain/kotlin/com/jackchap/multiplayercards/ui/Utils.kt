package com.jackchap.multiplayercards.ui

import korlibs.korge.view.*

fun Container.linearInterpolate(start: Double, end: Double, ratio: Float): Double {
    return start + (end - start) * ratio
}

