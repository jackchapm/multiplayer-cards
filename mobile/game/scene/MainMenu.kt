package scene

import Game
import godot.annotation.RegisterClass
import godot.annotation.RegisterFunction
import godot.api.BaseButton
import godot.api.Control
import godot.api.PackedScene
import godot.api.ResourceLoader
import godot.core.connect
import godot.coroutines.awaitMainThread
import godot.coroutines.godotCoroutine
import godot.extension.getNodeAs
import godot.extension.loadAs
import godot.global.GD
import io.ktor.client.call.*
import io.ktor.client.request.*
import network.HttpClient
import network.model.CreateGameRequest
import network.model.DeckType
import network.model.JoinGameRequest
import network.model.JoinGameResponse

// todo move create and join game logic to separate class
const val TABLE_PATH = "res://game/scene/table_scene.tscn"
const val CREATE_GAME_ENDPOINT = "/game/create"
const val JOIN_GAME_ENDPOINT = "/game/join"

@RegisterClass
class MainMenu : Control() {
	@RegisterFunction
	override fun _ready() {
		getNodeAs<BaseButton>("%JoinButton")?.pressed?.connect {
			godotCoroutine {
				val resp = HttpClient.client.post(JOIN_GAME_ENDPOINT) {
					setBody(JoinGameRequest(""))
				}.body<JoinGameResponse>()

				populateGameScene(resp)
			}
		}

		getNodeAs<BaseButton>("%NewButton")?.pressed?.connect {
			godotCoroutine {
				// todo handle errors here
				try {
					val resp = HttpClient.client.post(CREATE_GAME_ENDPOINT) {
						val r = CreateGameRequest("test game", DeckType.Standard)
						setBody(r)
					}.body<JoinGameResponse>()
					populateGameScene(resp)
				} catch (e: Exception) {
					GD.pushError(e)
				}
			}
		}
	}

	private suspend fun populateGameScene(response: JoinGameResponse) {
		awaitMainThread {
			val node = ResourceLoader.loadAs<PackedScene>(TABLE_PATH)?.instantiate() ?: return@awaitMainThread
			getTree()?.root?.addChild(node)
			val game = Game(response.gameId, response.token)
			node.addChild(game)
			free()
		}
	}

}
