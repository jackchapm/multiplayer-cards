package stack

import card.Card
import godot.annotation.Export
import godot.annotation.RegisterClass
import godot.annotation.RegisterFunction
import godot.annotation.RegisterProperty
import godot.api.*
import godot.core.VariantArray
import godot.core.Vector2
import godot.extension.connect
import godot.extension.instantiateAs
import godot.extension.loadAs

const val DRAG_INACCURACY = 1
const val STACK_THRESHOLD_SQUARED = 5000

@RegisterClass
class Stack : Area2D() {
	@RegisterProperty
	var stackId: String = "local"

	@RegisterProperty
	@Export
	var stackSize: Int = 1

	lateinit var cardParent: Node
	val cards: VariantArray<Node>
		get() = cardParent.getChildren()

	val indexes = mutableListOf<Int>()
	var dragging = false
	var dragDelta = Vector2.ZERO
	var dragStartPos = Vector2.ZERO
	var dragTarget: Node2D? = null


	@RegisterFunction
	override fun _ready() {
		cardParent = getNode("Cards")!!

		connect(inputEvent, this, this::onInput)
	}


	@RegisterFunction
	fun onInput(node: Viewport, input: InputEvent, idx: Long) {
		if (input is InputEventScreenTouch) {
			node.setInputAsHandled()
			// todo move to separate Draggable node?
			if (input.pressed) indexes += input.index

			if (input.pressed && !dragging) {
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
					1 -> flipTopCard()
					2 -> TODO("take card to hand")
				}
			} else {
				// handle drag drop (e.g stacking cards, sending websocket message)
				// todo send websocket messages
				if (dragTarget == this) {
					val closest = closestOverlapping(dragTarget!!)
					if(closest != null) {
						val newCards = closest.getNode("Cards")!!
						for(card in cards) {
							cardParent.removeChild(card)
							newCards.addChild(card)
						}
						queueFree()
					}
				} else {
					val spawnLocation = dragTarget!!.globalPosition
					val targetStack = closestOverlapping(dragTarget!!) ?: run {
						ResourceLoader.loadAs<PackedScene>(sceneFilePath)!!.instantiateAs<Stack>()!!.apply {
							this@Stack.getParent()?.addChild(this)
							globalPosition = spawnLocation
						}
					}

					cardParent.removeChild(dragTarget)
					targetStack.getNode("Cards")?.addChild(dragTarget)
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
		getParent()?.getChildren()
			?.filterIsInstance<Stack>()
			?.filterNot { it == this }
			?.associateWith { it.globalPosition.distanceSquaredTo(node.globalPosition) }
			?.filterValues { it < STACK_THRESHOLD_SQUARED }
			?.minByOrNull { it.value }
			?.key

	fun flipTopCard() {
		cards.mutate(cards.indices.last) {
			(it as? Card)?.flip()
		}
	}
}
