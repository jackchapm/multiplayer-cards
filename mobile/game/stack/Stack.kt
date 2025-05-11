package stack

import Game
import card.Card
import godot.annotation.*
import godot.api.*
import godot.core.Signal1
import godot.core.VariantArray
import godot.core.Vector2
import godot.extension.connect
import godot.extension.instantiateAs
import godot.extension.loadAs
import network.HttpClient
import network.model.StackState
import network.model.WebsocketMessage

const val DRAG_INACCURACY = 1
const val STACK_THRESHOLD_SQUARED = 5000

@RegisterClass
class Stack(
    @RegisterProperty @Export var stackId: String = "local"
) : Area2D() {
    companion object {
        private const val STACK_RESOURCE = "res://game/stack/stack.tscn"
        private val scene by lazy { ResourceLoader.loadAs<PackedScene>(STACK_RESOURCE)!! }

        fun StackState.instantiate(game: Game, after: (Stack) -> Unit = {}): Stack = this.let { state ->
            scene.instantiateAs<Stack>()!!.apply {
                stackId = state.stackId
                if (stackId != "local") {
                    uniqueNameInOwner = true
                    websocketSendSignal = game.websocketSend
                } else addToGroup("local")

                val cards = getNode("%Cards")!!

                if (remainingCards > 1) cards.addChild(Card.instantiate(0))

                cards.addChild(Card.instantiate(state.visibleCard.toInt()))

                globalPosition = state.position
                stackSize = state.remainingCards

                after(this)
            }
        }
    }

    @RegisterProperty
    @Export
    var stackSize: Int = 1

    lateinit var cardParent: Node
    val cards: VariantArray<Node>
        get() = cardParent.getChildren()

    lateinit var websocketSendSignal: Signal1<String>

    val indexes = mutableListOf<Int>()
    var dragging = false
    var dragDelta = Vector2.ZERO
    var dragStartPos = Vector2.ZERO
    var dragTarget: Node2D? = null

    @RegisterConstructor
    constructor() : this("local")

    @RegisterFunction
    override fun _ready() {
        cardParent = getNode("%Cards")!!
        if (stackId != "local") setName(stackId)

        connect(inputEvent, this, this::onInput)
    }

    // todo handle modifying stack before new stack is pushed to server
    // add to local group and check first on game state recieve?
    @RegisterFunction
    fun onInput(node: Viewport, input: InputEvent, idx: Long) {
        if (!input.isPressed()) return

        if (input is InputEventScreenTouch) {
            node.setInputAsHandled()
            // todo move to separate Draggable node?
            indexes += input.index

            if (!dragging) {
                dragging = true
                dragStartPos = input.position
                dragDelta = input.position
            }
        }

    }


    @RegisterFunction
    override fun _unhandledInput(event: InputEvent?) {
        if (!dragging) return
        if (event is InputEventScreenDrag && event.index in indexes) {
            getViewport()?.setInputAsHandled()
            if (dragTarget == null) {
                dragTarget = when (indexes.size) {
                    1 -> if (cards.size == 1) this else cards.popBack()
                    2 -> this
                    else -> null
                } as Node2D?

                // when a stack is moved, render it above other stacks
                dragTarget?.getParent()?.let {
                    it.moveChild(dragTarget, it.getChildCount() - 1)
                }

                // temporarily draw dragged stack over other nodes so it's easier to visualise
                dragTarget?.zIndex = 1
            }

            if (dragTarget == null) return
            // dragTarget should not be modified concurrently
            dragTarget!!.globalPosition += event.position - dragDelta
            dragDelta = event.position
        }

        if (event is InputEventScreenTouch && !event.pressed && event.index in indexes) {
            getViewport()?.setInputAsHandled()
            if (dragTarget == null || dragStartPos.distanceSquaredTo(event.position) < DRAG_INACCURACY) {
                // handle click
                when (indexes.size) {
                    1 -> {
                        flipTopCard()
                        tryEmit(WebsocketMessage.FlipCard(stackId))
                    }

                    2 -> {

                    }
                }
            } else {
                // handle drag drop (e.g stacking cards, sending websocket message)
                // todo send websocket messages
                if (dragTarget == this) {
                    val closest = closestOverlapping(dragTarget!!)
                    if (closest != null) {
                        tryEmit(WebsocketMessage.MoveStack(stackId, closest.globalPosition))
                        val newCards = closest.getNode("%Cards")!!
                        for (card in cards) {
                            cardParent.removeChild(card)
                            newCards.addChild(card)
                        }
                        queueFree()
                    } else {
                        tryEmit(WebsocketMessage.MoveStack(stackId, dragTarget!!.position))
                    }
                } else {
                    val spawnLocation = dragTarget!!.globalPosition
                    val closest = closestOverlapping(dragTarget!!)
                    tryEmit(WebsocketMessage.MoveCard(stackId, closest?.position ?: spawnLocation))
                    val targetStack = closest ?: scene.instantiateAs<Stack>()!!.apply {
                        stackId = "local"
                        stackSize = 1

                        this@Stack.getParent()?.addChild(this)
                        addToGroup("local")
                        globalPosition = spawnLocation
                    }

                    cardParent.removeChild(dragTarget)
                    targetStack.getNode("%Cards")?.addChild(dragTarget)
                    dragTarget?.position = Vector2.ZERO
                }

                dragTarget?.zIndex = 0
            }

            dragging = false
            dragTarget = null
            indexes.clear()
        }
    }

    fun closestOverlapping(node: Node2D) =
        getParent()?.getChildren()?.filterIsInstance<Stack>()?.filterNot { it == this }
            ?.associateWith { it.globalPosition.distanceSquaredTo(node.globalPosition) }
            ?.filterValues { it < STACK_THRESHOLD_SQUARED }?.minByOrNull { it.value }?.key

    fun flipTopCard() {
        cards.mutate(cards.indices.last) {
            (it as? Card)?.flip()
        }
    }

    fun tryEmit(message: WebsocketMessage) {
        if (::websocketSendSignal.isInitialized) websocketSendSignal.emit(HttpClient.json.encodeToString(message))
    }
}
