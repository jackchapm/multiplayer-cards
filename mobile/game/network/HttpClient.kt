package network

import godot.api.FileAccess
import godot.api.Node
import godot.api.OS
import godot.global.GD
import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.engine.cio.*
import io.ktor.client.plugins.*
import io.ktor.client.plugins.auth.*
import io.ktor.client.plugins.auth.providers.*
import io.ktor.client.plugins.contentnegotiation.*
import io.ktor.client.plugins.websocket.*
import io.ktor.client.request.*
import io.ktor.http.*
import io.ktor.serialization.kotlinx.*
import io.ktor.serialization.kotlinx.json.*
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.json.Json
import network.model.AuthResponse

const val BASE_URL = "https://cards.jackchap.com/"
const val REFRESH_TOKEN_PATH = "user://token"
const val AUTH_ENDPOINT = "/auth/guest"
const val REFRESH_ENDPOINT = "/auth/refresh"

class HttpClient : Node() {
    companion object {
        private val INSTANCE by lazy(LazyThreadSafetyMode.SYNCHRONIZED) { HttpClient() }

        val json = Json {
            isLenient = true
            encodeDefaults = true
            explicitNulls = false
        }

        val client
            get() = INSTANCE.httpClient
    }

    private val basicClient by lazy {
        HttpClient(CIO) {
            install(ContentNegotiation) {
                json(json)
            }


//            install(Logging) {
//                logger = object : Logger {
//                    override fun log(message: String) {
//                        GD.print(message)
//                    }
//                }
//                level = LogLevel.ALL
//            }

            defaultRequest {
                url(BASE_URL)
                contentType(ContentType.Application.Json)
            }
        }
    }

    val httpClient by lazy {
        basicClient.config {
            install(WebSockets) {
                contentConverter = KotlinxWebsocketSerializationConverter(Json)
                pingIntervalMillis = 60000
            }

            install(Auth) {
                reAuthorizeOnResponse { it.status.value in 401..403 }

                bearer {
                    loadTokens {
                        // don't send tokens for websocket
                        // todo move /auth/guest into its own scene so user can be prompted for display name etc.
                        val refreshToken = FileAccess.getFileAsString(REFRESH_TOKEN_PATH)

                        // If refresh token file doesn't exist, authenticate user for the first time
                        if (refreshToken.isBlank()) {
                            //todo handle error (not authresponse)
                            val resp = basicClient.post(AUTH_ENDPOINT).body<AuthResponse>()
                            saveToken(resp)
                            BearerTokens(resp.accessToken, resp.refreshToken)
                        } else {
                            val split = refreshToken.split(',', limit=2)
                            BearerTokens(split[0], split[1])
                        }
                    }
                    refreshTokens {
                        val resp = basicClient.post(REFRESH_ENDPOINT) {
                            bearerAuth(oldTokens?.refreshToken ?: "")
                            markAsRefreshTokenRequest()
                        }.body<AuthResponse>()
                        saveToken(resp)
                        BearerTokens(resp.accessToken, resp.refreshToken)
                    }
                    sendWithoutRequest {
                        // don't try to send an unauthenticated request first
                        it.url.pathSegments.getOrNull(1) == "game"
                    }
                }
            }
        }
    }

    private fun saveToken(response: AuthResponse) {
        with(FileAccess.open(REFRESH_TOKEN_PATH, FileAccess.ModeFlags.WRITE)) {
            if (this?.storeString("${response.accessToken},${response.refreshToken}") != true) {
                GD.pushError("Failed to save refresh token")
            }
        }
    }
}
