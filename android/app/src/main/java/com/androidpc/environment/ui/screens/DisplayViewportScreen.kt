package com.androidpc.environment.ui.screens

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import java.nio.ByteBuffer

@Composable
fun DisplayViewportScreen(
    displayAddress: String = "127.0.0.1",
    displayPort: Int = 5901,
    onDisconnect: () -> Unit
) {
    var cursorPosition by remember { mutableStateOf(Offset(400f, 300f)) }
    var isLeftDown by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf("Connecting to RFB display server at $displayAddress:$displayPort...") }
    var showVirtualKeyboard by remember { mutableStateOf(true) }

    var displayWidth by remember { mutableIntStateOf(1280) }
    var displayHeight by remember { mutableIntStateOf(720) }
    var bitmap by remember {
        mutableStateOf(Bitmap.createBitmap(1280, 720, Bitmap.Config.ARGB_8888))
    }
    var frameCount by remember { mutableLongStateOf(0L) }

    // Connect RFB stream and run active rendering loop
    LaunchedEffect(displayAddress, displayPort) {
        val (w, h) = NativeBridge.connectDisplayClient(displayAddress, displayPort)
        displayWidth = w
        displayHeight = h
        bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
        statusMessage = "Connected: Desktop Session Active [${w}x${h} RFB 003.008]"

        val byteBuffer = ByteBuffer.allocateDirect(w * h * 4)
        val pixelArray = ByteArray(w * h * 4)

        withContext(Dispatchers.Default) {
            while (isActive) {
                val copied = NativeBridge.getFramebuffer(pixelArray)
                if (copied > 0) {
                    byteBuffer.rewind()
                    byteBuffer.put(pixelArray, 0, copied)
                    byteBuffer.rewind()
                    bitmap.copyPixelsFromBuffer(byteBuffer)
                    frameCount++
                }
                delay(33) // ~30 FPS frame sync
            }
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
    ) {
        // Desktop Viewport Canvas with Touch-to-Mouse Coordinate Translation
        Canvas(
            modifier = Modifier
                .fillMaxSize()
                .pointerInput(Unit) {
                    detectTapGestures(
                        onTap = { offset ->
                            cursorPosition = offset
                            val normX = ((offset.x / size.width) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
                            val normY = ((offset.y / size.height) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
                            NativeBridge.sendPointer(1, normX, normY)
                            NativeBridge.sendPointer(0, normX, normY)
                            statusMessage = "Click: ($normX, $normY) | Res: ${displayWidth}x${displayHeight}"
                        },
                        onLongPress = { offset ->
                            cursorPosition = offset
                            val normX = ((offset.x / size.width) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
                            val normY = ((offset.y / size.height) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
                            NativeBridge.sendPointer(4, normX, normY) // Right Click
                            NativeBridge.sendPointer(0, normX, normY)
                            statusMessage = "Context Click: ($normX, $normY)"
                        }
                    )
                }
                .pointerInput(Unit) {
                    detectDragGestures(
                        onDragStart = { offset ->
                            cursorPosition = offset
                            isLeftDown = true
                            val normX = ((offset.x / size.width) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
                            val normY = ((offset.y / size.height) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
                            NativeBridge.sendPointer(1, normX, normY)
                        },
                        onDrag = { change, dragAmount ->
                            change.consume()
                            cursorPosition += dragAmount
                            val normX = ((cursorPosition.x / size.width) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
                            val normY = ((cursorPosition.y / size.height) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
                            NativeBridge.sendPointer(1, normX, normY)
                            statusMessage = "Drag: ($normX, $normY)"
                        },
                        onDragEnd = {
                            isLeftDown = false
                            val normX = ((cursorPosition.x / size.width) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
                            val normY = ((cursorPosition.y / size.height) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
                            NativeBridge.sendPointer(0, normX, normY)
                        }
                    )
                }
        ) {
            // Draw decoded RFB frame buffer
            // Referencing frameCount ensures recomposition on every frame update
            if (frameCount >= 0) {
                drawImage(
                    image = bitmap.asImageBitmap(),
                    dstSize = IntSize(size.width.toInt(), size.height.toInt())
                )
            }

            // Draw interactive Desktop Cursor
            drawCircle(
                color = if (isLeftDown) AccentCyan else Color.White,
                radius = 8f,
                center = cursorPosition
            )
            drawCircle(
                color = Color.Black,
                radius = 8f,
                center = cursorPosition,
                style = androidx.compose.ui.graphics.drawscope.Stroke(width = 1.5f)
            )
        }

        // Top Status Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp)
                .background(SurfaceDark.copy(alpha = 0.85f), RoundedCornerShape(8.dp))
                .border(1.dp, CardBorder, RoundedCornerShape(8.dp))
                .padding(horizontal = 14.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    modifier = Modifier
                        .size(8.dp)
                        .background(StatusGreen, RoundedCornerShape(4.dp))
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = statusMessage,
                    style = MaterialTheme.typography.labelSmall,
                    color = TextPrimary
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(
                    onClick = { showVirtualKeyboard = !showVirtualKeyboard },
                    colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                    shape = RoundedCornerShape(6.dp),
                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                    modifier = Modifier.height(30.dp)
                ) {
                    Text("Keys", style = MaterialTheme.typography.labelSmall, color = TextPrimary)
                }

                Button(
                    onClick = {
                        NativeBridge.disconnectDisplayClient()
                        onDisconnect()
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = StatusRed.copy(alpha = 0.8f)),
                    shape = RoundedCornerShape(6.dp),
                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                    modifier = Modifier.height(30.dp)
                ) {
                    Text("Exit", style = MaterialTheme.typography.labelSmall, color = Color.White)
                }
            }
        }

        // Floating Virtual Keyboard Bar (Desktop Special Keys)
        if (showVirtualKeyboard) {
            val keySymMap = mapOf(
                "Esc" to 0xff1bL,
                "Tab" to 0xff09L,
                "Ctrl" to 0xffe3L,
                "Alt" to 0xffe9L,
                "Super" to 0xffebL,
                "Enter" to 0xff0dL,
                "↑" to 0xff52L,
                "↓" to 0xff54L,
                "←" to 0xff51L,
                "→" to 0xff53L
            )

            Row(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .fillMaxWidth()
                    .padding(bottom = 48.dp, start = 8.dp, end = 8.dp)
                    .background(SurfaceDark.copy(alpha = 0.95f), RoundedCornerShape(10.dp))
                    .border(1.dp, CardBorder, RoundedCornerShape(10.dp))
                    .padding(6.dp),
                horizontalArrangement = Arrangement.SpaceEvenly
            ) {
                keySymMap.forEach { (key, sym) ->
                    Button(
                        onClick = {
                            NativeBridge.sendKey(1, sym)
                            NativeBridge.sendKey(0, sym)
                            statusMessage = "Key Dispatched: $key [0x${java.lang.Long.toHexString(sym)}]"
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(34.dp)
                    ) {
                        Text(key, style = MaterialTheme.typography.labelSmall, color = AccentCyan, fontWeight = FontWeight.Bold)
                    }
                }
            }
        }
    }
}
