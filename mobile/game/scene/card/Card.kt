package scene.card

import godot.annotation.Export
import godot.annotation.RegisterClass
import godot.annotation.RegisterFunction
import godot.annotation.RegisterProperty
import godot.api.*
import godot.core.Vector2
import godot.extension.connect
import godot.extension.getNodeAs
import godot.extension.loadAs

const val TEXTURE_PATH = "res://game/scene/card/assets/card_front_frames.tres"
const val DRAG_INACCURACY = 1

@RegisterClass
class Card : Area2D() {
	@RegisterProperty
	@Export
	var cardValue: Int = 0
		set(value) {
			field = value
			if (isInsideTree()) updateTexture()
		}

	val textureResource by lazy { ResourceLoader.loadAs<SpriteFrames>(TEXTURE_PATH) }
	var dragging = false
	var dragDelta = Vector2.ZERO
	var dragStartPos = Vector2.ZERO

	lateinit var sprite2D: Sprite2D

	@RegisterFunction
	override fun _ready() {
		sprite2D = getNodeAs("Sprite2D")!!
		updateTexture()

		connect(inputEvent, this, this::onInput)
	}


	@RegisterFunction
	fun onInput(node: Node, input: InputEvent, flags: Long) {
		if (input is InputEventScreenTouch) {
			if (input.index != 0) return

			// todo move to separate Draggable node?
			if (input.pressed && !dragging) {
				dragging = true
				dragStartPos = input.position
				dragDelta = input.position
			}

			// todo keep track of inputted index and only allow a single at a time, but of any index
			// e.g player can tap a card whilst holding another with index 0
			// but this event shouldn't be triggered if more than one index
			if (!input.pressed && dragging) {
				dragging = false
				if (dragStartPos.distanceSquaredTo(input.position) < DRAG_INACCURACY) {
					// handle click
					flip()
				} else {
					// handle drag drop (e.g stacking cards)
				}
			}
		}

	}

	@RegisterFunction
	override fun _input(event: InputEvent?) {
		if (event is InputEventScreenDrag && dragging) {
			if (event.index != 0) return
			this.globalPosition += event.position - dragDelta
			dragDelta = event.position
		}
	}

	fun updateTexture() {
		// todo: Convert to Atlas rather than SpriteFrames (after custom card sheet made)
		val textureId = cardValue.takeIf { it and 0b1000_0000 == 0 }?.let {
			val suit = it and 0b11
			val rank = it ushr 2
			// Rank is 1-indexed so we should subtract 1 here
			// however, frame 0 is the back of card frame, so we have to offset by 1 anyway
			suit * 13 + rank
		} ?: 0
		sprite2D.texture = textureResource?.getFrameTexture("default", textureId)
	}


	fun flip() {
		cardValue = cardValue xor 0b1000_0000
	}
}
