package com.jackchap.multiplayercards.network

import com.jackchap.multiplayercards.models.*
import io.ktor.client.*
import io.ktor.client.plugins.websocket.*
import io.ktor.http.*
import io.ktor.websocket.*
import korlibs.io.net.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.*

class GameWebSocketClient(
    private val baseUrl: String,
    private val token: String,
    private val gameId: String,
    private val playerId: String
) {
    private val json = Json { ignoreUnknownKeys = true }
    private val client = HttpClient {
        install(WebSockets)
    }

    private var session: WebSocketSession? = null
    private val _gameState = MutableStateFlow<WebSocketResponse.GameState?>(null)
    val gameState = _gameState.asStateFlow()

    private val _playerState = MutableStateFlow<WebSocketResponse.PlayerState?>(null)
    val playerState = _playerState.asStateFlow()

    private val _errorMessages = MutableSharedFlow<String>()
    val errorMessages = _errorMessages.asSharedFlow()

    private var pingJob: Job? = null

    suspend fun connect() {
        try {
            session = client.webSocketSession {
                url {
                    protocol = URLProtocol.WSS
                    host = baseUrl
                    path("game")
                    parameters.append("token", token)
                    parameters.append("gameId", gameId)
                    parameters.append("playerId", playerId)
                }
            }

            joinGame()
            startPingPong()

            session?.incoming?.consumeAsFlow()?.collect { frame ->
                if (frame is Frame.Text) {
                    processResponse(frame.readText())
                }
            }
        } catch (e: Exception) {
            _errorMessages.emit("WebSocket error: ${e.message}")
            delay(5000)
            connect() // Try to reconnect
        }
    }

    private fun startPingPong() {
        pingJob = CoroutineScope(Dispatchers.Default).launch {
            while (isActive) {
                delay(30000) // Send ping every 30 seconds
                sendPing()
            }
        }
    }

    private suspend fun processResponse(text: String) {
        when (val response = json.decodeFromString<WebSocketResponse>(text)) {
            is WebSocketResponse.GameState -> _gameState.emit(response)
            is WebSocketResponse.PlayerState -> _playerState.emit(response)
            is WebSocketResponse.Error -> _errorMessages.emit(response.message)
            is WebSocketResponse.CloseGame -> disconnect()
            else -> {} // Handle other responses
        }
    }

    suspend fun disconnect() {
        pingJob?.cancel()
        session?.close()
        client.close()
    }

    suspend fun joinGame() {
        sendRequest(WebSocketRequest.JoinGame())
    }

    suspend fun takeCard(stackId: String) {
        sendRequest(WebSocketRequest.TakeCard(stackId))
    }

    suspend fun putCard(handIndex: Int, position: Pair<Int, Int>, faceDown: Boolean) {
        sendRequest(WebSocketRequest.PutCard(handIndex, position, faceDown))
    }

    suspend fun flipCard(stackId: String) {
        sendRequest(WebSocketRequest.FlipCard(stackId))
    }

    suspend fun flipStack(stackId: String) {
        sendRequest(WebSocketRequest.FlipStack(stackId))
    }

    suspend fun moveCard(stackId: String, position: Pair<Int, Int>) {
        sendRequest(WebSocketRequest.MoveCard(stackId, position))
    }

    suspend fun moveStack(stackId: String, position: Pair<Int, Int>) {
        sendRequest(WebSocketRequest.MoveStack(stackId, position))
    }

    suspend fun shuffle(stackId: String) {
        sendRequest(WebSocketRequest.Shuffle(stackId))
    }

    suspend fun reset() {
        sendRequest(WebSocketRequest.Reset())
    }

    suspend fun leaveGame() {
        sendRequest(WebSocketRequest.LeaveGame())
    }

    private suspend fun sendPing() {
        sendRequest(WebSocketRequest.Ping())
    }

    private suspend fun sendRequest(request: WebSocketRequest) {
        session?.send(json.encodeToString(request))
    }
}
