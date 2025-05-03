package com.jackchap.multiplayercards.scenes

import com.jackchap.multiplayercards.models.*
import com.jackchap.multiplayercards.network.GameWebSocketClient
import com.jackchap.multiplayercards.ui.HandView
import com.jackchap.multiplayercards.ui.StackView
import korlibs.image.format.*
import korlibs.io.file.std.*
import korlibs.korge.input.*
import korlibs.korge.scene.*
import korlibs.korge.view.*
import korlibs.korge.view.align.*
import korlibs.math.geom.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.collectLatest

class GameScene(
    private val websocketClient: GameWebSocketClient
) : Scene() {

    private val stacks = mutableMapOf<String, StackView>()
    private lateinit var handView: HandView
    private val tableContainer = Container()
    private val uiContainer = Container()
    private val cardAssets = mutableMapOf<String, ImageData>()

    override suspend fun SContainer.sceneMain() {
        // Load all card assets
        loadCardAssets()

        // Setup UI layout
        addChild(tableContainer)
        addChild(uiContainer)

        // Initialize hand view at bottom of screen
        handView = HandView().addTo(uiContainer)
        handView.position(0, views().virtualHeight - HandView.HAND_HEIGHT)
        handView.width = views().virtualWidth.toDouble()

        // Listen for game state updates
        launch {
            websocketClient.gameState.collectLatest { gameState ->
                if (gameState != null) {
                    updateGameState(gameState)
                }
            }
        }

        // Listen for player state updates
        launch {
            websocketClient.playerState.collectLatest { playerState ->
                if (playerState != null) {
                    updatePlayerState(playerState)
                }
            }
        }

        // Handle error messages
        launch {
            websocketClient.errorMessages.collectLatest { errorMessage ->
                // Show error message to user
                text(errorMessage) {
                    position(views().virtualWidth / 2, 50.0)
                    centerXOnStage()

                    // Fade out after 3 seconds
                    launch {
                        delay(3000)
                        removeFromParent()
                    }
                }
            }
        }

        // Connect to WebSocket
        launch {
            websocketClient.connect()
        }
    }

    private suspend fun loadCardAssets() {
        // Load card back
        cardAssets["back"] = resourcesVfs["cards/red_backing.png"].readImageData()

        // Load all other card images
        val suits = listOf("spades", "hearts", "diamonds", "clubs")
        val ranks = listOf("ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king")

        for (suit in suits) {
            for (rank in ranks) {
                val path = "cards/${rank}_of_$suit.png"
                try {
                    cardAssets[path] = resourcesVfs[path].readImageData()
                } catch (e: Exception) {
                    println("Failed to load asset: $path")
                }
            }
        }

        // Load jokers
        cardAssets["cards/joker_black.png"] = resourcesVfs["cards/joker_black.png"].readImageData()
        cardAssets["cards/joker_red.png"] = resourcesVfs["cards/joker_red.png"].readImageData()
    }

    private fun updateGameState(gameState: WebSocketResponse.GameState) {
        // Remove stacks that are no longer in the game
        val currentStackIds = gameState.stacks.map { it.stackId }.toSet()
        val stacksToRemove = stacks.keys.filter { it !in currentStackIds }

        for (stackId in stacksToRemove) {
            stacks[stackId]?.removeFromParent()
            stacks.remove(stackId)
        }

        // Update or add stacks
        for (stackState in gameState.stacks) {
            val stackView = stacks.getOrPut(stackState.stackId) {
                StackView(stackState.stackId).apply {
                    tableContainer.addChild(this)
                    setupStackInteractions(this)
                }
            }

            // Update stack properties
            stackView.updateStack(
                stackState.position.first,
                stackState.position.second,
                stackState.visibleCard,
                stackState.remainingCards,
                cardAssets
            )
        }
    }

    private fun updatePlayerState(playerState: WebSocketResponse.PlayerState) {
        handView.updateHand(playerState.hand, cardAssets)
    }

    private fun setupStackInteractions(stackView: StackView) {
        stackView.onClick { e ->
            // Check if this is a multi-touch event (two fingers)
            if (e.input.touch.activeTouches.size > 1) {
                // Handle two-finger tap - take card to hand
                launch {
                    websocketClient.takeCard(stackView.stackId)
                }
            } else {
                // Single tap to flip card
                launch {
                    val currentCard = stackView.getVisibleCard()
                    // Create a flipped version of the card
                    val flippedCard = Card(currentCard.value xor Card.FACE_DOWN_MASK)
                    // Animate the flip locally for responsiveness
                    stackView.topCard.animateFlip(flippedCard, cardAssets)
                    // Send the flip request to server
                    websocketClient.flipCard(stackView.stackId)
                }
            }
        }

        // Dragging stack
        var startDragPos = Point.ZERO
        var startStackPos = Point.ZERO

        stackView.onDown { e ->
            startDragPos = e.currentPosLocal
            startStackPos = stackView.pos
        }

        stackView.onMove { e ->
            if (e.pressing) {
                val dx = e.currentPosLocal.x - startDragPos.x
                val dy = e.currentPosLocal.y - startDragPos.y

                stackView.position(
                    startStackPos.x + dx,
                    startStackPos.y + dy
                )
            }
        }

        stackView.onUp { e ->
            // Convert to grid coordinates
            val gridX = (stackView.x / 100).toInt()
            val gridY = (stackView.y / 100).toInt()

            launch {
                websocketClient.moveStack(stackView.stackId, Pair(gridX, gridY))
            }
        }
    }
}
