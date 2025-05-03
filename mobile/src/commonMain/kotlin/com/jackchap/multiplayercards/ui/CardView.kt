package com.jackchap.multiplayercards.ui

import com.jackchap.multiplayercards.models.Card
import korlibs.image.bitmap.*
import korlibs.image.format.ImageData
import korlibs.korge.tween.*
import korlibs.korge.view.*
import korlibs.math.geom.*
import korlibs.math.interpolation.*
import korlibs.time.*

class CardView : Container() {
    companion object {
        const val CARD_WIDTH = 140.0
        const val CARD_HEIGHT = 190.0
    }

    private val cardImage = Image(Bitmap32(1, 1, premultiplied = true)).addTo(this)

    init {
        width = CARD_WIDTH
        height = CARD_HEIGHT

        cardImage.size(CARD_WIDTH, CARD_HEIGHT)
        cardImage.smoothing = false
    }

    fun updateCard(card: Card, assets: Map<String, ImageData>) {
        val imagePath = card.getImagePath()
        val bitmap = assets[imagePath]?.mainBitmap ?: assets["cards/red_backing.png"]?.mainBitmap

        bitmap?.let {
            cardImage.bitmap = it
        }
    }

    suspend fun animateFlip(newCard: Card, assets: Map<String, ImageData>) {
        val startScale = scale

        // First half of flip animation - scale X to 0
        tween(
            time = 150.milliseconds,
            callback = { ratio ->
                scaleX = linearInterpolate(startScale.toRatio().value, 0.0, ratio)
            }
        )

        // Update card when flipped
        updateCard(newCard, assets)

        // Second half of flip animation - scale X back from 0 to 1
        tween(
            time = 150.milliseconds,
            callback = { ratio ->
                scaleX = linearInterpolate(0.0, startScale.toRatio().value, ratio)
            }
        )
    }
}
