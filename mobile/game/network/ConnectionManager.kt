package network

import Game
import godot.annotation.RegisterClass
import godot.annotation.RegisterConstructor
import godot.annotation.RegisterFunction
import godot.api.Node
import godot.coroutines.awaitMainThread
import godot.coroutines.godotCoroutine
import godot.global.GD
import io.ktor.client.plugins.websocket.*
import io.ktor.client.request.*
import io.ktor.websocket.*
import kotlinx.coroutines.isActive
import network.model.CauseAction
import network.model.WebsocketMessage
import network.model.WebsocketResponse
import stack.Stack

const val WEBSOCKET_ENDPOINT = "wss://cardsws.jackchap.com"

@RegisterClass
class ConnectionManager(private val token: String) : Node() {

    @RegisterConstructor
    constructor() : this("")

    private lateinit var websocket: DefaultClientWebSocketSession

    lateinit var game: Game

    @RegisterFunction
    override fun _ready() = godotCoroutine {
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
                        is WebsocketResponse.Error -> {}
                        is WebsocketResponse.CloseGame -> {}
                        is WebsocketResponse.Success -> {}
                        is WebsocketResponse.Pong -> {}
                    }
                }
            } catch (e: Exception) {
                GD.pushError(e)
            }
        }
    }

    private fun handleGameState(state: WebsocketResponse.GameState) {
        GD.print("received frame: ${state.causeAction}")
        when (state.causeAction) {
            CauseAction.Ping -> {
                game.stacks.getChildren().forEach(Node::queueFree)
            }

            CauseAction.PopCard if (state.causePlayer == "us" || true) -> {
                val oldStack = state.stacks!![0]
                val newStack = state.stacks[1]
                val localStack = getTree()!!.getNodesInGroup("moving").firstOrNull {
                    it.name.toString() == "moving${oldStack.stackId}"
                } as Stack?
                localStack?.apply {
                    setName(newStack.stackId)
                    uniqueNameInOwner = true
                    stackId = newStack.stackId
                    websocketSendSignal = game.websocketSend
                }
            }

            CauseAction.DropStack if (state.causePlayer == "us" || true) -> {
                // don't remove from moving group until after drop stack frame is received to avoid flickering
                (getTree()!!.getNodesInGroup("moving")
                    .firstOrNull { it.name.toString() == state.stacks!![0].stackId } as Stack?)?.removeFromGroup("moving")
            }

            else -> {}
        }

        // todo add boolean isNew to stack state
        state.stacks?.forEach(game::replaceOrCreate)
        getTree()!!.getNodesInGroup("local").forEach {
            if (it.isInGroup("moving")) return@forEach
            queueFree()
        }
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

