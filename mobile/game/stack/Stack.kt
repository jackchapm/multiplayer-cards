package stack

import card.Card
import godot.annotation.*
import godot.api.*
import godot.core.Signal1
import godot.core.VariantArray
import godot.core.Vector2
import godot.core.connect
import godot.extension.connect
import godot.extension.getNodeAs
import godot.extension.instantiateAs
import godot.extension.loadAs
import network.HttpClient
import network.model.WebsocketMessage
import util.STACK_RESOURCE

const val DRAG_INACCURACY = 1
const val STACK_THRESHOLD_SQUARED = 5000

@RegisterClass
class Stack(
    @RegisterProperty @Export var stackId: String = "local"
) : Area2D() {
    companion object {
        private val scene by lazy { ResourceLoader.loadAs<PackedScene>(STACK_RESOURCE)!! }

        fun instantiate(block: (Stack.() -> Unit) = {}) = scene.instantiateAs<Stack>()!!.apply { block() }
    }

    @RegisterProperty
    @Export
    var stackSize: Int = 1

    val cardParent by lazy { getNode("%Cards")!! }
    val cards: VariantArray<Node>
        get() = cardParent.getChildren()

    lateinit var dragTimer: Timer
    lateinit var panelControl: Panel

    var websocketSendSignal: Signal1<String>? = null
        set(value) {
            field = value
            emitQueue()
        }

    private val queuedMessages = mutableListOf<String>()

    val indexes = mutableListOf<Int>()
    var dragging = false
    var dragDelta = Vector2.ZERO
    var dragStartPos = Vector2.ZERO

    @RegisterConstructor
    constructor() : this("local")

    @RegisterFunction
    override fun _ready() {
        dragTimer = getNodeAs("DragTimer")!!
        panelControl = getNodeAs("Panel")!!
        if (stackId != "local") setName(stackId)

        dragTimer.timeout.connect {
            if (isInGroup("local")) tryEmit(WebsocketMessage.StackMessage.MoveStack(stackId, globalPosition), false)
        }

        panelControl.connect(panelControl.guiInput, this, this::onInput)
//		connect(panelControl.guiInput, this, this::onInput)
    }

    // todo handle modifying stack before new stack is pushed to server
    // add to local group and check first on game state receive?
    @RegisterFunction
    fun onInput(input: InputEvent) {
        if (!input.isPressed()) return
        if (input is InputEventScreenTouch) {
            panelControl.acceptEvent()
            val pos = getCanvasTransform() * (globalTransform * (input.position + panelControl.position))
            // todo move to separate Draggable node?
            if (!dragging) startDragging(pos)

            indexes += input.index
        }

    }

    fun startDragging(pos: Vector2) {
        dragging = true
        dragStartPos = pos
        dragDelta = pos
        indexes.clear()
        dragTimer.start()
    }

    fun stopDragging() {
        dragging = false
        indexes.clear()
        dragTimer.stop()
    }

    @RegisterFunction
    override fun _unhandledInput(event: InputEvent?) {
        if (!dragging) return
        if (event is InputEventScreenDrag && event.index in indexes) {
            getViewport()?.setInputAsHandled()
            if (indexes.size == 1 && stackSize > 1 && !isInGroup("moving")) {
                val card = cards.popBack()
                cardParent.removeChild(card)
                scene.instantiateAs<Stack>()!!.let {
                    it.setName("moving${stackId}")

                    getParent()!!.addChild(it)
                    it.addToGroup("local")
                    it.addToGroup("moving")

                    it.getNode("%Cards")!!.addChild(card)
                    it.globalPosition = globalPosition + event.position - dragDelta

                    // move state to new stack
                    it.startDragging(dragStartPos)
                    it.indexes.addAll(indexes)
                }
                stopDragging()
                tryEmit(WebsocketMessage.StackMessage.PopCard(stackId))
                return
            }
            zIndex = 1
            addToGroup("local")
            addToGroup("moving")
            globalPosition += event.position - dragDelta
            dragDelta = event.position
        } else if (event is InputEventScreenTouch && !event.pressed && event.index in indexes) {
            getViewport()?.setInputAsHandled()
            val indexCount = indexes.size
            stopDragging()
            if (dragStartPos == dragDelta || dragStartPos.distanceSquaredTo(globalPosition) < DRAG_INACCURACY) {
                // handle click
                when (indexCount) {
                    1 -> {
                        flipTopCard()
                        tryEmit(WebsocketMessage.StackMessage.FlipCard(stackId))
                    }

                    2 -> {

                    }
                }
            } else {
                val closest = closestOverlapping()
                if (closest != null) {
                    tryEmit(WebsocketMessage.StackMessage.DropStack(stackId, closest.globalPosition))
                    for (card in cards) {
                        cardParent.removeChild(card)
                        closest.cardParent.addChild(card)
                    }
                    queueFree()
                } else {
                    zIndex = 0
                    // if a stack is moved to a new location, bring it to top
                    getParent()?.let {
                        it.moveChild(this, it.getChildCount() - 1)
                    }
                    tryEmit(WebsocketMessage.StackMessage.DropStack(stackId, globalPosition))
                }
            }
        }
    }

    fun closestOverlapping() = getParent()?.getChildren()?.filterIsInstance<Stack>()?.filterNot { it == this }
        ?.associateWith { it.globalPosition.distanceSquaredTo(globalPosition) }
        ?.filterValues { it < STACK_THRESHOLD_SQUARED }?.minByOrNull { it.value }?.key

    fun flipTopCard() {
        cards.mutate(cards.indices.last) {
            (it as? Card)?.flip()
        }
    }

    fun changeTopCard(card: Int) {
        cardParent.getChildren().forEach(Node::free)
        if (stackSize > 1) cardParent.addChild(Card.instantiate(0))
        cardParent.addChild(Card.instantiate(card))
    }

    private fun tryEmit(message: WebsocketMessage, queue: Boolean = true) {
        val json = HttpClient.json.encodeToString(message)
        if (websocketSendSignal == null && queue) queuedMessages.add(json)
        else websocketSendSignal?.emit(json)
    }

    private fun emitQueue() {
        // todo look into https://github.com/arrow-kt/arrow to do this without serialising first
        websocketSendSignal?.let {
            queuedMessages.removeAll { msg ->
                it.emit(msg.replace(""""stack":"local"""", """"stack":"$stackId""""))
                true
            }
        }
    }
}
