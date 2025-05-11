package network.model

import godot.annotation.RegisterClass
import godot.api.Object
import godot.core.Vector2
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class WebsocketMessage {
    @Serializable
    @SerialName("join-game")
    data object JoinGame : WebsocketMessage()

    @Serializable
    @SerialName("take-card")
    data class TakeCard(val stack: StackId) : WebsocketMessage()

    @Serializable
    @SerialName("put-card")
    data class PutCard(
        val handIndex: Int,
        @Serializable(with = Vector2Serializer::class) val position: Vector2,
        val faceDown: Boolean
    ) : WebsocketMessage()

    @Serializable
    @SerialName("flip-card")
    data class FlipCard(val stack: StackId) : WebsocketMessage()

    @Serializable
    @SerialName("flip-stack")
    data class FlipStack(val stack: StackId) : WebsocketMessage()

    @Serializable
    @SerialName("move-card")
    data class MoveCard(
        val stack: StackId,
        @Serializable(with = Vector2Serializer::class) val position: Vector2
    ) : WebsocketMessage()

    @Serializable
    @SerialName("move-stack")
    data class MoveStack(
        val stack: StackId,
        @Serializable(with = Vector2Serializer::class) val position: Vector2
    ) : WebsocketMessage()

    @Serializable
    @SerialName("shuffle")
    data class Shuffle(val stack: StackId) : WebsocketMessage()

    @Serializable
    @SerialName("deal")
    data class Deal(val stack: StackId) : WebsocketMessage()

    @Serializable
    @SerialName("give-player")
    data class GivePlayer(val handIndex: Int, val tradeTo: PlayerId) : WebsocketMessage()

    @Serializable
    @SerialName("reset")
    data object Reset : WebsocketMessage()

    @Serializable
    @SerialName("leave-game")
    data object LeaveGame : WebsocketMessage()

    @Serializable
    @SerialName("ping")
    data object Ping : WebsocketMessage()
}
