package network.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class DeckType {
    @Serializable
    @SerialName("standard")
    data object Standard : DeckType()

    @Serializable
    @SerialName("custom")
    data class Custom(val stacks: List<List<Int>>) : DeckType()
}

@Serializable
data class CreateGameRequest(
    val name: String,
    val deckType: DeckType,
)

@Serializable
data class JoinGameRequest(
    val gameId: String,
)

@Serializable
data class JoinGameResponse(
    val gameId: String,
    val token: String,
)
