package network.model

import godot.core.Vector2
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder

object Vector2Serializer : KSerializer<Vector2> {
    private val delegateSerializer = ListSerializer(Int.serializer())
    override val descriptor: SerialDescriptor = delegateSerializer.descriptor

    override fun serialize(encoder: Encoder, value: Vector2) {
        val list = listOf(value.x.toInt(), value.y.toInt())
        encoder.encodeSerializableValue(delegateSerializer, list)
    }

    override fun deserialize(decoder: Decoder): Vector2 {
        // Deserialize JSON array [x, y] into Vector2
        val list = decoder.decodeSerializableValue(delegateSerializer)
        if (list.size == 2) {
            return Vector2(list[0], list[1])
        } else {
            throw SerializationException("Expected a JSON array with 2 elements for Vector2, but found ${list.size} elements.")
        }
    }
}
