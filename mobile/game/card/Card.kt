package card

import godot.annotation.Export
import godot.annotation.RegisterClass
import godot.annotation.RegisterFunction
import godot.annotation.RegisterProperty
import godot.api.*
import godot.extension.getNodeAs
import godot.extension.instantiateAs
import godot.extension.loadAs

const val TEXTURE_PATH = "res://game/card/assets/card_front_frames.tres"

@RegisterClass
class Card : Area2D() {
    companion object {
        private const val CARD_RESOURCE = "res://game/card/card.tscn"
        private val scene by lazy { ResourceLoader.loadAs<PackedScene>(CARD_RESOURCE)!! }

        fun instantiate(value: Int): Card = scene.instantiateAs<Card>()!!.apply {
            cardValue = value
        }
    }

    @Export
    @RegisterProperty
    var cardValue: Int = 0
        set(value) {
            field = value
            if (isInsideTree()) updateTexture()
        }

    @RegisterProperty
    lateinit var sprite2D: Sprite2D

    private val textureResource by lazy { ResourceLoader.loadAs<SpriteFrames>(TEXTURE_PATH) }

    @RegisterFunction
    override fun _ready() {
        sprite2D = getNodeAs("Sprite2D")!!
        updateTexture()
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
