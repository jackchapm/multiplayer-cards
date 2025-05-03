package com.jackchap.multiplayercards.ui

import com.jackchap.multiplayercards.models.Card
import com.jackchap.multiplayercards.ui.CardView.Companion.CARD_HEIGHT
import korlibs.image.color.*
import korlibs.image.format.ImageData
import korlibs.io.async.*
import korlibs.korge.input.*
import korlibs.korge.tween.get
import korlibs.korge.tween.tween
import korlibs.korge.view.*
import korlibs.math.geom.*
import korlibs.math.interpolation.*
import kotlinx.coroutines.launch
import kotlin.time.Duration.Companion.milliseconds

class HandView : Container() {
    companion object {
        const val HAND_HEIGHT = 120.0
        const val CARD_SPACING = 40.0
        const val VISIBLE_CARD_HEIGHT = 80.0
    }

    private val cardContainer = Container().addTo(this)
    private val cardViews = mutableListOf<CardView>()

    init {
        // Semi-transparent background for hand area
        graphics {
            fill(Colors["#00000099"]) {
                rect(0.0, 0.0, 2000.0, HAND_HEIGHT)
            }
        }
    }

    fun updateHand(cards: List<Card>, assets: Map<String, ImageData>) {
        val oldCount = cardViews.size

        // Add new cards
        for (i in oldCount until cards.size) {
            val cardView = CardView().apply {
                position(width / 2, -CARD_HEIGHT)
                updateCard(cards[i], assets)
            }
            cardContainer.addChild(cardView)
            cardViews.add(cardView)

            // Animate card into hand
            launch {
//                cardView.tween(
//                    time = 500.0.milliseconds,
//                    easing = Easing.EASE_OUT_ELASTIC,
//                    callback = { ratio ->
//                        val targetX = calculateCardPosition(cards.size, i)
//                        cardView.x = linearInterpolate(cardView.width / 2, targetX, ratio)
//                        cardView.y = linearInterpolate(-CardView.CARD_HEIGHT, HAND_HEIGHT - VISIBLE_CARD_HEIGHT, ratio)
//                    }
//                )
                cardView.tween(
                    cardView::x[calculateCardPosition(cards.size, i)],
                    cardView::y[HAND_HEIGHT - VISIBLE_CARD_HEIGHT],
                    time = 500.milliseconds,
                    easing = Easing.EASE_OUT_ELASTIC
                )
            }
        }

        // Remove excess cards with animation
        while (cardViews.size > cards.size) {
            val cardToRemove = cardViews.removeAt(cardViews.size - 1)

            launch {
                cardToRemove.tween(
                    cardToRemove::y[HAND_HEIGHT + CARD_HEIGHT],
                    cardToRemove::alpha[0.0],
                    time = 300.milliseconds,
                    easing = Easing.EASE_IN,
                    callback = {
                        cardToRemove.removeFromParent()
                    }
                )
            }
        }

        // Update existing cards
        for (i in 0 until minOf(oldCount, cards.size)) {
            val cardView = cardViews[i]
            cardView.updateCard(cards[i], assets)

            // Reposition card if needed
            val targetX = calculateCardPosition(cards.size, i)
            if (cardView.x != targetX) {
                launch {
                    cardView.tween(
                        cardView::x[targetX],
                        time = 300.0.milliseconds,
                        easing = Easing.EASE_OUT,
                    )
                }
            }
        }

        // Setup card interactions
        for (i in 0 until cardViews.size) {
            setupCardInteraction(cardViews[i], i)
        }
    }

    private fun calculateCardPosition(totalCards: Int, index: Int): Double {
        val totalWidth = minOf(totalCards * CARD_SPACING, width)
        val leftMargin = (width - totalWidth) / 2
        return leftMargin + index * CARD_SPACING + CardView.CARD_WIDTH / 2
    }

    private fun setupCardInteraction(cardView: CardView, index: Int) {
        cardView.onOver {
            launch {
                cardView.tween(
                    cardView::y[HAND_HEIGHT - VISIBLE_CARD_HEIGHT - 30],
                    time = 200.0.milliseconds,
                )
            }
        }

        cardView.onOut {
            launch {
                cardView.tween(
                    cardView::y[HAND_HEIGHT - VISIBLE_CARD_HEIGHT],
                    time = 200.0.milliseconds,
                )
            }
        }

        var startDragPos = Point.ZERO
        var isDragging = false

        cardView.onDown {
            startDragPos = it.currentPosLocal
            cardView.bringToTop()
        }

        cardView.onMove {
            if (it.pressing && it.currentPosLocal.distanceTo(startDragPos) > 10) {
                isDragging = true
                cardView.position(
                    cardView.x + (it.currentPosLocal.x - it.lastPosLocal.x),
                    cardView.y + (it.currentPosLocal.y - it.lastPosLocal.y)
                )
            }
        }

        cardView.onUp {
            if (isDragging) {
                // Convert to grid coordinates for putting on table
                val gridPos = localToGlobal(cardView.pos)
                val parent = cardView.parent
                val gridX = (gridPos.x / 100).toInt()
                val gridY = (gridPos.y / 100).toInt()

                // If dropped outside hand area
                if (gridPos.y < ((parent?.height?.minus(HAND_HEIGHT)) ?: 0.0)) {
                    // Remove from hand view temporarily
                    cardView.removeFromParent()
                    cardViews.remove(cardView)

                    // Put card on table
                    launch {
                        stage?.let { stage ->
                            // Determine if card should be face down based on orientation
                            // val faceDown = it.multitouch.allTouches.size > 1

                            // Add to stage temporarily to show animation
                            stage.addChild(cardView)
                            cardView.position(gridPos)

                            websocketClient.putCard(index, Pair(gridX, gridY), cardView.)
                        }
                    }
                } else {
                    // Return to position in hand with animation
                    launch {
                        cardView.tween(
                            time = 200.0.milliseconds,
                            easing = Easing.EASE_OUT,
                            callback = { ratio ->
                                val targetX = calculateCardPosition(cardViews.size, cardViews.indexOf(cardView))
                                cardView.x = linearInterpolate(cardView.x, targetX, ratio)
                                cardView.y = linearInterpolate(cardView.y, HAND_HEIGHT - VISIBLE_CARD_HEIGHT, ratio)
                            }
                        )
                    }
                }
                isDragging = false
            }
        }
    }
}
