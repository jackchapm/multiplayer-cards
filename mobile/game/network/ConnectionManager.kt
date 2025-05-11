package network

import Game
import godot.annotation.RegisterClass
import godot.annotation.RegisterConstructor
import godot.annotation.RegisterFunction
import godot.api.Node
import godot.api.PackedScene
import godot.api.ResourceLoader
import godot.coroutines.awaitMainThread
import godot.coroutines.godotCoroutine
import godot.extension.getNodeAs
import godot.extension.loadAs
import godot.global.GD
import io.ktor.client.plugins.websocket.*
import io.ktor.client.request.*
import io.ktor.websocket.*
import kotlinx.coroutines.isActive
import network.model.CauseAction
import network.model.WebsocketMessage
import network.model.WebsocketResponse
import stack.Stack
import stack.Stack.Companion.instantiate

const val WEBSOCKET_ENDPOINT = "wss://cardsws.jackchap.com"
const val CARD_RESOURCE = "res://game/card/card.tscn"

@RegisterClass
class ConnectionManager(private val token: String) : Node() {

    @RegisterConstructor
    constructor() : this("")

    private lateinit var websocket: DefaultClientWebSocketSession

    private lateinit var stackScene: PackedScene
    private lateinit var cardScene: PackedScene

    lateinit var game: Game

    @RegisterFunction
    override fun _ready() = godotCoroutine {
        cardScene = ResourceLoader.loadAs(CARD_RESOURCE)!!

        game = getParent() as Game

        websocket = HttpClient.client.webSocketSession(WEBSOCKET_ENDPOINT) {
            bearerAuth(token)
        }

        websocket.sendSerialized(WebsocketMessage.JoinGame as WebsocketMessage)
        while (websocket.isActive) {
            try {
                val frame = websocket.receiveDeserialized<WebsocketResponse>()
                awaitMainThread {
                    when (frame) {
                        is WebsocketResponse.GameState -> handleGameState(frame)
                        is WebsocketResponse.PlayerState -> handlePlayerState(frame)
                        is WebsocketResponse.Error -> {

                        }

                        is WebsocketResponse.CloseGame -> {

                        }

                        is WebsocketResponse.Success -> {

                        }

                        is WebsocketResponse.Pong -> {

                        }
                    }
                }
            } catch (e: Exception) {
                GD.pushError(e)
            }
        }
    }

    private fun handleGameState(state: WebsocketResponse.GameState) {
        GD.print(state)
        when (state.causeAction) {
            CauseAction.Ping -> {
                game.stacks.getChildren().forEach(Node::queueFree)
            }
            else -> {}
        }

        getTree()!!.getNodesInGroup("local").forEach(Node::queueFree)

        // todo add boolean isNew to stack state
        // todo mutate rather than replace
        state.stacks?.forEach { state -> game.replaceOrCreate(state) }
    }

    private fun handlePlayerState(state: WebsocketResponse.PlayerState) {
        //todo
        GD.print("player state: $state")
    }

    suspend fun sendMessage(message: WebsocketMessage) {
        websocket.sendSerialized(message)
    }

    suspend fun sendMessageString(message: String) {
        websocket.send(message)
    }
}

