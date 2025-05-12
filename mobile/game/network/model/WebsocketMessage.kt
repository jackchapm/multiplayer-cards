package network.model

import godot.core.Vector2
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class WebsocketMessage {
    @Serializable
    sealed class StackMessage : WebsocketMessage() {
        abstract val stack: StackId

        @Serializable
        @SerialName("take-card")
        data class TakeCard(override val stack: StackId) : StackMessage()

        @Serializable
        @SerialName("flip-card")
        data class FlipCard(override val stack: StackId) : StackMessage()

        @Serializable
        @SerialName("flip-stack")
        data class FlipStack(override val stack: StackId) : StackMessage()

        @Serializable
        @SerialName("pop-card")
        data class PopCard(override val stack: StackId) : StackMessage()

        @Serializable
        @SerialName("move-stack")
        data class MoveStack(
            override val stack: StackId, @Serializable(with = Vector2Serializer::class) val position: Vector2
        ) : StackMessage()

        @Serializable
        @SerialName("drop-stack")
        data class DropStack(
            override val stack: StackId, @Serializable(with = Vector2Serializer::class) val position: Vector2
        ) : StackMessage()

        @Serializable
        @SerialName("shuffle")
        data class Shuffle(override val stack: StackId) : StackMessage()

        @Serializable
        @SerialName("deal")
        data class Deal(override val stack: StackId) : StackMessage()
    }

    @Serializable
    @SerialName("join-game")
    data object JoinGame : WebsocketMessage()

    @Serializable
    @SerialName("put-card")
    data class PutCard(
        val handIndex: Int, @Serializable(with = Vector2Serializer::class) val position: Vector2, val faceDown: Boolean
    ) : WebsocketMessage()


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
