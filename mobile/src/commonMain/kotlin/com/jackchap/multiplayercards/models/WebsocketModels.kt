package com.jackchap.multiplayercards.models

import com.jackchap.multiplayercards.models.Card
import com.jackchap.multiplayercards.models.StackState
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class WebSocketRequest {
    @Serializable
    @SerialName("join-game")
    class JoinGame : WebSocketRequest()

    @Serializable @SerialName("take-card")
    data class TakeCard(val stack: String) : WebSocketRequest()

    @Serializable @SerialName("put-card")
    data class PutCard(
        val handIndex: Int,
        val position: Pair<Int, Int>,
        val faceDown: Boolean
    ) : WebSocketRequest()

    @Serializable @SerialName("flip-card")
    data class FlipCard(val stack: String) : WebSocketRequest()

    @Serializable @SerialName("flip-stack")
    data class FlipStack(val stack: String) : WebSocketRequest()

    @Serializable @SerialName("move-card")
    data class MoveCard(val stack: String, val position: Pair<Int, Int>) : WebSocketRequest()

    @Serializable @SerialName("move-stack")
    data class MoveStack(val stack: String, val position: Pair<Int, Int>) : WebSocketRequest()

    @Serializable @SerialName("shuffle")
    data class Shuffle(val stack: String) : WebSocketRequest()

    @Serializable @SerialName("reset")
    class Reset : WebSocketRequest()

    @Serializable @SerialName("leave-game")
    class LeaveGame : WebSocketRequest()

    @Serializable @SerialName("ping")
    class Ping : WebSocketRequest()
}

@Serializable
sealed class WebSocketResponse {
    @Serializable @SerialName("game-state")
    data class GameState(
        val gameId: String,
        val owner: String,
        val connectedPlayers: List<String>,
        val stacks: List<StackState>
    ) : WebSocketResponse()

    @Serializable @SerialName("player-state")
    data class PlayerState(
        val gameId: String,
        val hand: List<Card>
    ) : WebSocketResponse()

    @Serializable @SerialName("error")
    data class Error(val error: String, val message: String) : WebSocketResponse()

    @Serializable @SerialName("close-game")
    object CloseGame : WebSocketResponse()

    @Serializable @SerialName("success")
    object Success : WebSocketResponse()

    @Serializable @SerialName("no-response")
    object NoResponse : WebSocketResponse()

    @Serializable @SerialName("pong")
    object Pong : WebSocketResponse()
}

@Serializable
data class StackState(
    val stackId: String,
    val position: Pair<Int, Int>,
    val visibleCard: Card,
    val remainingCards: Int
)
