package com.jackchap.multiplayercards.models

import kotlinx.serialization.Serializable

@Serializable
data class Card(val value: Int) {
    companion object {
        const val HIDDEN_CARD: Int = 0b1000_0000
        const val SPECIAL_MASK: Int = 0b0100_0000
        const val FACE_DOWN_MASK: Int = 0b1000_0000
    }

    fun isFaceDown(): Boolean = (value and FACE_DOWN_MASK) != 0
    fun isSpecial(): Boolean = (value and SPECIAL_MASK) != 0
    fun isNumerical(): Boolean = (value and SPECIAL_MASK) == 0

    fun rank(): Int? = if (isNumerical()) (value shr 2) else null
    fun suit(): Int? = if (isNumerical()) (value and 0b11) else null
    fun specialType(): Int? = if (isSpecial()) (value and 0b0011_1111) else null

    fun getImagePath(): String {
        if (isFaceDown()) {
            return "cards/red_backing.png"
        }

        return if (isSpecial()) {
            when (specialType()) {
                0 -> "cards/joker_black.png"
                1 -> "cards/joker_red.png"
                else -> "cards/red_backing.png"
            }
        } else {
            val suitName = when (suit()) {
                0 -> "spades"
                1 -> "hearts"
                2 -> "diamonds"
                3 -> "clubs"
                else -> "unknown"
            }

            val rankName = when (rank()) {
                1 -> "ace"
                11 -> "jack"
                12 -> "queen"
                13 -> "king"
                else -> rank().toString()
            }

            "cards/${rankName}_of_$suitName.png"
        }
    }
}
