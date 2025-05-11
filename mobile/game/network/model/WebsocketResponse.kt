package network.model

import godot.core.Vector2
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
enum class CauseAction {
    @SerialName("join-game") JoinGame,
    @SerialName("take-card") TakeCard,
    @SerialName("put-card") PutCard,
    @SerialName("flip-card") FlipCard,
    @SerialName("move-card") MoveCard,
    @SerialName("move-stack") MoveStack,
    @SerialName("shuffle") Shuffle,
    @SerialName("deal") Deal,
    @SerialName("give-player") GivePlayer,
    @SerialName("reset") Reset,
    @SerialName("leave-game") LeaveGame,
    @SerialName("ping") Ping
}

@Serializable
data class StackState(
    val stackId: StackId,
    @Serializable(with = Vector2Serializer::class) val position: Vector2,
    val visibleCard: Int,
    val remainingCards: Int,
)

@Serializable
sealed class WebsocketResponse {
    @Serializable
    @SerialName("game-state")
    data class GameState(
        val gameId: GameId,
        val causeAction: CauseAction?,
        val causePlayer: PlayerId?,
        val owner: PlayerId?,
        val players: List<PlayerId>?,
        val stacks: List<StackState>?,
    ) : WebsocketResponse()

    @Serializable
    @SerialName("player-state")
    data class PlayerState(
        val gameId: GameId,
        val hand: List<UInt>,
    ) : WebsocketResponse()

    @Serializable
    @SerialName("error")
    data class Error(
        val error: String,
        val message: String,
    ) : WebsocketResponse()

    @Serializable @SerialName("close-game") data object CloseGame : WebsocketResponse()

    @Serializable data object Success : WebsocketResponse()
    @Serializable data object Pong : WebsocketResponse()
}
