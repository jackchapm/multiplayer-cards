package scene

import godot.annotation.RegisterClass
import godot.annotation.RegisterFunction
import godot.api.BaseButton
import godot.api.VBoxContainer
import godot.core.connect
import godot.extension.getNodeAs

const val TABLE_PATH = "res://game/scene/table_scene.tscn"

@RegisterClass
class ButtonContainer : VBoxContainer() {
    @RegisterFunction
    override fun _ready() {
        // todo handle this error
        val joinGame: BaseButton = getNodeAs("JoinButton") ?: return
        joinGame.pressed.connect {
            getTree()?.changeSceneToFile(TABLE_PATH)
        }
    }
}
