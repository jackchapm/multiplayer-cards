package com.jackchap.multiplayercards.ui

import com.jackchap.multiplayercards.models.Card
import korlibs.image.format.ImageData
import korlibs.korge.tween.*
import korlibs.korge.view.*
import korlibs.math.geom.*
import korlibs.math.interpolation.*
import kotlin.time.Duration.Companion.milliseconds

class StackView(val stackId: String) : Container() {
    private val cards = arrayListOf<CardView>()
    var topCard = CardView().addTo(this)
        private set
    private val countText = Text("", 16.0).addTo(this)

    private var currentVisibleCard: Card = Card(Card.HIDDEN_CARD)

    fun getVisibleCard(): Card = currentVisibleCard

    init {
        countText.position(5, 5)
        cards.add(topCard)
    }

    suspend fun updateStack(
        gridX: Int,
        gridY: Int,
        visibleCard: Card,
        count: Int,
        assets: Map<String, ImageData>
    ) {
        // Animate to new position if needed
        val targetX = gridX * 100.0
        val targetY = gridY * 100.0

        if (x != targetX || y != targetY) {
            tween(
                time = 300.0.milliseconds,
                easing = Easing.EASE_OUT,
                callback = { ratio ->
                    x = linearInterpolate(x, targetX, ratio)
                    y = linearInterpolate(y, targetY, ratio)
                }
            )
        }

        // Update visible card
        topCard.updateCard(visibleCard, assets)
        currentVisibleCard = visibleCard

        // Update count
        countText.text = if (count > 1) count.toString() else ""

        // Add visual offset for stack with multiple cards
        if (count > 1) {
            for (i in cards.size until minOf(count, 5)) {
                val offsetCard = CardView().apply {
                    alpha = 0.6
                    position(i * 3.0, i * 3.0)
                    updateCard(Card(Card.HIDDEN_CARD), assets)
                }
                addChildAt(offsetCard, 0)
                cards.add(offsetCard)
            }
        }

        // Remove excess cards
        while (cards.size > maxOf(count, 1)) {
            val card = cards.removeAt(0)
            card.removeFromParent()
        }
    }
}
