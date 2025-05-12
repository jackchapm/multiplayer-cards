import godot.annotation.RegisterClass
import godot.annotation.RegisterConstructor
import godot.annotation.RegisterFunction
import godot.annotation.RegisterSignal
import godot.api.Node
import godot.api.Node2D
import godot.core.VariantArray
import godot.core.connect
import godot.core.signal1
import godot.core.variantArrayOf
import godot.coroutines.godotCoroutine
import godot.extension.getNodeAs
import godot.global.GD
import network.ConnectionManager
import network.model.GameId
import network.model.PlayerId
import network.model.StackState
import stack.Stack

@RegisterClass
class Game(val gameId: GameId, val token: String) : Node() {
    // todo should be converted to WebsocketMessage pending https://github.com/utopia-rise/godot-kotlin-jvm/issues/488
    @RegisterSignal
    val websocketSend by signal1<String>()

    lateinit var connectionManager: ConnectionManager
    lateinit var stacks: Node2D

    var players: VariantArray<PlayerId> = variantArrayOf()

    @RegisterConstructor
    constructor() : this("", "")

    @RegisterFunction
    override fun _ready() {
        setName("Game")
        uniqueNameInOwner = true

        connectionManager = ConnectionManager(token)
        stacks = Node2D().apply {
            setName("Stacks")
            uniqueNameInOwner = true
        }

        addChild(connectionManager)
        addChild(stacks)

        websocketSend.connect {
            godotCoroutine {
                GD.print("sending frame: $it")
                connectionManager.sendMessageString(it)
            }
        }
    }

    fun replaceOrCreate(stackState: StackState) {
        val stackPath = stackState.stackId
        if (stacks.hasNode(stackPath)) {
            val stack = stacks.getNodeAs<Stack>(stackPath)!!
            if(stackState.remainingCards == 0) {
                stack.free()
                return
            }
            val wasLocal = stack.isInGroup("local")
            if(wasLocal) stack.removeFromGroup("local")
            if (stack.globalPosition != stackState.position && (!wasLocal || !stack.isInGroup("moving"))) {
                stack.globalPosition = stackState.position
                stacks.moveChild(stack, stacks.getChildCount() - 1)
            }
            stack.stackSize = stackState.remainingCards
            stack.changeTopCard(stackState.visibleCard)
        } else if (stackState.remainingCards > 0) {
            Stack.instantiate {
                stackId = stackState.stackId
                uniqueNameInOwner = true
                websocketSendSignal = websocketSend

                stackSize = stackState.remainingCards
                changeTopCard(stackState.visibleCard)
                globalPosition = stackState.position

                stacks.addChild(this)
            }
        }
    }

}
