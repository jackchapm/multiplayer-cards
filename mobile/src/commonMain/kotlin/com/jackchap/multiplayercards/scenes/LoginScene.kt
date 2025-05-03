//package com.jackchap.multiplayercards.scenes
//
//import com.jackchap.multiplayercards.models.DeckType
//import com.jackchap.multiplayercards.network.GameService
//import korlibs.event.Key
//import korlibs.korge.input.keys
//import korlibs.korge.input.onClick
//import korlibs.korge.scene.Scene
//import korlibs.korge.ui.*
//import korlibs.korge.view.*
//import korlibs.korge.view.align.*
//import korlibs.math.geom.*
//import kotlinx.coroutines.launch
//
//class LoginScene : Scene() {
//
//    private lateinit var gameIdInput: UITextInput
//    private lateinit var playerNameInput: UITextInput
//    private lateinit var statusText: Text
//    private val gameService = GameService()
//
//    override suspend fun SContainer.sceneMain() {
//        val uiContainer = uiContainer {
//            width = views.virtualWidth.toDouble()
//            height = views.virtualHeight.toDouble()
//        }
//
//        uiContainer.uiVerticalStack(width = 500.0) {
//            centerXOnStage()
//            y = 100.0
//
//            uiText("Multiplayer Cards") {
//                textSize = 32.0
//            }
//
//            uiSpacing(height = 20.0)
//
//            uiText("Player Name:")
//            playerNameInput = uiTextInput {
//                text = "Player"
//            }
//
//            uiSpacing(height = 10.0)
//
//            uiText("Game ID (leave empty to create new):")
//            gameIdInput = uiTextInput {
//                text = ""
//
//                keys {
//                    down(Key.ENTER) {
//                        joinOrCreateGame()
//                    }
//                }
//            }
//
//            uiSpacing(height = 20.0)
//
//            uiHorizontalStack {
//                uiButton("Create Game") {
//                    onClick {
//                        createNewGame()
//                    }
//                }
//
//                uiSpacing(width = 20.0)
//
//                uiButton("Join Game") {
//                    onClick {
//                        joinExistingGame()
//                    }
//                }
//            }
//
//            uiSpacing(height = 20.0)
//
//            statusText = uiText("") {
//                textColor = Colors.RED
//            }
//        }
//    }
//
//    private fun joinOrCreateGame() {
//        val gameId = gameIdInput.text.trim()
//        if (gameId.isEmpty()) {
//            createNewGame()
//        } else {
//            joinExistingGame()
//        }
//    }
//
//    private fun createNewGame() {
//        val playerName = playerNameInput.text.trim()
//        if (playerName.isEmpty()) {
//            statusText.text = "Please enter a player name"
//            return
//        }
//
//        statusText.text = "Creating new game..."
//
//        launch {
//            try {
//                val response = gameService.createGame(playerName, DeckType.STANDARD)
//
//                // Navigate to game scene
//                sceneContainer.changeTo({
//                    GameScene(
//                        websocketClient = gameService.connectToGame(
//                            response.gameId,
//                            response.token
//                        )
//                    )
//                })
//            } catch (e: Exception) {
//                statusText.text = "Error: ${e.message}"
//            }
//        }
//    }
//
//    private fun joinExistingGame() {
//        val gameId = gameIdInput.text.trim()
//        val playerName = playerNameInput.text.trim()
//
//        if (gameId.isEmpty()) {
//            statusText.text = "Please enter a game ID"
//            return
//        }
//
//        if (playerName.isEmpty()) {
//            statusText.text = "Please enter a player name"
//            return
//        }
//
//        statusText.text = "Joining game..."
//
//        launch {
//            try {
//                val response = gameService.joinGame(gameId, playerName)
//
//                // Navigate to game scene
//                sceneContainer.changeTo({
//                    GameScene(
//                        websocketClient = gameService.connectToGame(
//                            response.gameId,
//                            response.token
//                        )
//                    )
//                })
//            } catch (e: Exception) {
//                statusText.text = "Error: ${e.message}"
//            }
//        }
//    }
//}
