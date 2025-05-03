import com.jackchap.multiplayercards.network.GameWebSocketClient
import com.jackchap.multiplayercards.scenes.GameScene
import korlibs.image.color.*
import korlibs.korge.Korge
import korlibs.korge.scene.*
import korlibs.korge.view.Views
import korlibs.math.geom.*

suspend fun main() = Korge(
    windowSize = Size(1280, 720),
    //width = 1280,
    //height = 720,
    title = "Multiplayer Cards",
    backgroundColor = Colors["#2b2b2b"]
) {
    // Normally you would get these from login/game creation flow
    val token = "player-token-from-auth"
    val gameId = "game-id-from-join-or-create"
    val playerId = "player-id"
    val wsUrl = "your-api-gateway-url"

    val websocketClient = GameWebSocketClient(
        baseUrl = wsUrl,
        token = token,
        gameId = gameId,
        playerId = playerId
    )

    val sceneContainer = sceneContainer()
    addChild(sceneContainer)

    sceneContainer.changeTo { GameScene(websocketClient) }
    Unit
}
