package com.androidpc.environment.ui.screens

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.service.AudioBridgePlayer
import com.androidpc.environment.service.LinuxRuntimeService
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.nio.ByteBuffer

@Composable
fun DisplayViewportScreen(
    displayAddress: String = "127.0.0.1",
    displayPort: Int = 5901,
    targetWidth: Int = 1280,
    targetHeight: Int = 720,
    audioEnabled: Boolean = true,
    onDisconnect: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var cursorPosition by remember { mutableStateOf(Offset(400f, 300f)) }
    var isLeftDown by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf("Initializing display server & RFB stream...") }
    var showVirtualKeyboard by remember { mutableStateOf(true) }
    var ctrlHeld by remember { mutableStateOf(false) }
    var altHeld by remember { mutableStateOf(false) }

    var displayWidth by remember { mutableIntStateOf(targetWidth) }
    var displayHeight by remember { mutableIntStateOf(targetHeight) }
    var bitmap by remember {
        mutableStateOf(Bitmap.createBitmap(targetWidth, targetHeight, Bitmap.Config.ARGB_8888))
    }
    var frameCount by remember { mutableLongStateOf(0L) }

    DisposableEffect(Unit) {
        LinuxRuntimeService.start(context, "Desktop Viewport ($displayAddress:$displayPort)")
        if (audioEnabled) {
            AudioBridgePlayer.start(scope, 4713)
        }
        onDispose {
            AudioBridgePlayer.stop(scope)
        }
    }

    // Start display server if available, connect RFB stream, and run active rendering loop
    LaunchedEffect(displayAddress, displayPort, targetWidth, targetHeight) {
        if (!NativeBridge.isDisplayRunning()) {
            NativeBridge.startDisplayServer(
                displayNum = 1,
                width = targetWidth,
                height = targetHeight,
                dpi = 160,
                port = displayPort
            )
        }

        val (w, h) = NativeBridge.connectDisplayClient(displayAddress, displayPort)
        displayWidth = w.coerceAtLeast(1)
        displayHeight = h.coerceAtLeast(1)
        bitmap = Bitmap.createBitmap(displayWidth, displayHeight, Bitmap.Config.ARGB_8888)
        statusMessage = if (NativeBridge.isDisplayRunning()) {
            "Live Xvnc [${displayWidth}x${displayHeight} RFB 003.008]"
        } else {
            "Offline Preview [${displayWidth}x${displayHeight} RFB 003.008]"
        }

        val byteBuffer = ByteBuffer.allocateDirect(displayWidth * displayHeight * 4)
        val pixelArray = ByteArray(displayWidth * displayHeight * 4)

        withContext(Dispatchers.Default) {
            while (isActive) {
                val copied = NativeBridge.getFramebuffer(pixelArray)
                if (copied > 0) {
                    byteBuffer.rewind()
                    byteBuffer.put(pixelArray, 0, copied.coerceAtMost(pixelArray.size))
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
        Canvas(
            modifier = Modifier
                .fillMaxSize()
                .pointerInput(displayWidth, displayHeight) {
                    detectTapGestures(
                        onTap = { offset ->
                            cursorPosition = offset
                            val normX = ((offset.x / size.width) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
                            val normY = ((offset.y / size.height) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
                            NativeBridge.sendPointer(1, normX, normY)
                            NativeBridge.sendPointer(0, normX, normY)
                            statusMessage = "Click: ($normX, $normY) | ${displayWidth}x${displayHeight}"
                        },
                        onLongPress = { offset ->
                            cursorPosition = offset
                            val normX = ((offset.x / size.width) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
                            val normY = ((offset.y / size.height) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
                            NativeBridge.sendPointer(4, normX, normY)
                            NativeBridge.sendPointer(0, normX, normY)
                            statusMessage = "Right Click: ($normX, $normY)"
                        }
                    )
                }
                .pointerInput(displayWidth, displayHeight) {
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
            if (frameCount >= 0) {
                drawImage(
                    image = bitmap.asImageBitmap(),
                    dstSize = IntSize(size.width.toInt(), size.height.toInt())
                )
            }

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
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                Box(
                    modifier = Modifier
                        .size(8.dp)
                        .background(StatusGreen, RoundedCornerShape(4.dp))
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = statusMessage,
                    style = MaterialTheme.typography.labelSmall,
                    color = TextPrimary,
                    maxLines = 1
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
                        LinuxRuntimeService.stop(context)
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

        // Floating Virtual Keyboard Bar (with sticky Ctrl / Alt modifiers)
        if (showVirtualKeyboard) {
            val keySymMap = listOf(
                "Esc" to 0xff1bL,
                "Tab" to 0xff09L,
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
                    .padding(bottom = 32.dp, start = 8.dp, end = 8.dp)
                    .background(SurfaceDark.copy(alpha = 0.95f), RoundedCornerShape(10.dp))
                    .border(1.dp, CardBorder, RoundedCornerShape(10.dp))
                    .padding(6.dp)
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp)
            ) {
                Button(
                    onClick = {
                        ctrlHeld = !ctrlHeld
                        NativeBridge.sendKey(if (ctrlHeld) 1 else 0, 0xffe3L)
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = if (ctrlHeld) AccentCyan else SurfaceContainer
                    ),
                    shape = RoundedCornerShape(6.dp),
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                    modifier = Modifier.height(34.dp)
                ) {
                    Text("Ctrl", style = MaterialTheme.typography.labelSmall, color = if (ctrlHeld) BgDark else AccentCyan, fontWeight = FontWeight.Bold)
                }

                Button(
                    onClick = {
                        altHeld = !altHeld
                        NativeBridge.sendKey(if (altHeld) 1 else 0, 0xffe9L)
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = if (altHeld) AccentCyan else SurfaceContainer
                    ),
                    shape = RoundedCornerShape(6.dp),
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                    modifier = Modifier.height(34.dp)
                ) {
                    Text("Alt", style = MaterialTheme.typography.labelSmall, color = if (altHeld) BgDark else AccentCyan, fontWeight = FontWeight.Bold)
                }

                keySymMap.forEach { (key, sym) ->
                    Button(
                        onClick = {
                            NativeBridge.sendKey(1, sym)
                            NativeBridge.sendKey(0, sym)
                            statusMessage = "Key: $key [0x${java.lang.Long.toHexString(sym)}]"
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                        modifier = Modifier.height(34.dp)
                    ) {
                        Text(key, style = MaterialTheme.typography.labelSmall, color = AccentCyan, fontWeight = FontWeight.Bold)
                    }
                }
            }
        }
    }
}
