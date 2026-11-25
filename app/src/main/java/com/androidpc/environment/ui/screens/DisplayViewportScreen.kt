package com.androidpc.environment.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Paint
import android.graphics.Rect
import android.view.KeyEvent as AndroidKeyEvent
import android.view.SurfaceHolder
import android.view.Surface
import android.view.SurfaceView
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.isSecondaryPressed
import androidx.compose.ui.input.pointer.isTertiaryPressed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.service.AudioBridgePlayer
import com.androidpc.environment.service.LinuxRuntimeService
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicInteger
import kotlin.coroutines.coroutineContext

enum class DesktopInputMode(val label: String) {
    DirectTouch("Touch"),
    Trackpad("Trackpad"),
    Relative("Relative")
}

private suspend fun presentWithCanvas(surface: Surface, surfaceLock: Any, isCurrent: () -> Boolean) {
    var bitmap: Bitmap? = null
    var pixels: ByteBuffer? = null
    var generation = 0L
    val paint = Paint(Paint.FILTER_BITMAP_FLAG)
    val destination = Rect()
    while (coroutineContext.isActive && surface.isValid && isCurrent()) {
        val (width, height) = NativeBridge.getDisplayDimensions()
        if (width <= 0 || height <= 0) return
        if (bitmap?.width != width || bitmap.height != height) {
            bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
            pixels = ByteBuffer.allocateDirect(width * height * 4)
            generation = 0
        }
        val image = checkNotNull(bitmap)
        val buffer = checkNotNull(pixels)
        buffer.rewind()
        val next = NativeBridge.getFramebufferDirect(buffer, generation)
        // A DesktopSize update can arrive in the same poll as the frame copy.
        if (next > 0 && NativeBridge.getDisplayDimensions() == (width to height)) {
            image.copyPixelsFromBuffer(buffer)
            synchronized(surfaceLock) {
                if (!surface.isValid || !isCurrent()) return
                try {
                    val canvas = surface.lockCanvas(null)
                    try {
                        destination.set(0, 0, canvas.width, canvas.height)
                        canvas.drawBitmap(image, null, destination, paint)
                    } finally {
                        surface.unlockCanvasAndPost(canvas)
                    }
                    generation = next
                } catch (_: IllegalArgumentException) {
                    // A Surface can become invalid inside the platform.
                    if (!surface.isValid) return
                } catch (_: Surface.OutOfResourcesException) {
                    return
                }
            }
        }
        delay(1)
    }
}

private fun androidCharToX11KeySym(ch: Char): Int {
    return when (ch) {
        '\n', '\r' -> 0xff0d
        '\t' -> 0xff09
        '\b' -> 0xff08
        else -> ch.code
    }
}

private fun androidKeyCodeToX11KeySym(keyCode: Int, unicodeChar: Int): Int {
    return when (keyCode) {
        AndroidKeyEvent.KEYCODE_ENTER, AndroidKeyEvent.KEYCODE_NUMPAD_ENTER -> 0xff0d
        AndroidKeyEvent.KEYCODE_DEL -> 0xff08
        AndroidKeyEvent.KEYCODE_FORWARD_DEL -> 0xffff
        AndroidKeyEvent.KEYCODE_TAB -> 0xff09
        AndroidKeyEvent.KEYCODE_ESCAPE -> 0xff1b
        AndroidKeyEvent.KEYCODE_DPAD_UP -> 0xff52
        AndroidKeyEvent.KEYCODE_DPAD_DOWN -> 0xff54
        AndroidKeyEvent.KEYCODE_DPAD_LEFT -> 0xff51
        AndroidKeyEvent.KEYCODE_DPAD_RIGHT -> 0xff53
        AndroidKeyEvent.KEYCODE_HOME -> 0xff50
        AndroidKeyEvent.KEYCODE_MOVE_END -> 0xff57
        AndroidKeyEvent.KEYCODE_PAGE_UP -> 0xff55
        AndroidKeyEvent.KEYCODE_PAGE_DOWN -> 0xff56
        AndroidKeyEvent.KEYCODE_CTRL_LEFT -> 0xffe3
        AndroidKeyEvent.KEYCODE_CTRL_RIGHT -> 0xffe4
        AndroidKeyEvent.KEYCODE_ALT_LEFT -> 0xffe9
        AndroidKeyEvent.KEYCODE_ALT_RIGHT -> 0xffea
        AndroidKeyEvent.KEYCODE_SHIFT_LEFT -> 0xffe1
        AndroidKeyEvent.KEYCODE_SHIFT_RIGHT -> 0xffe2
        AndroidKeyEvent.KEYCODE_META_LEFT, AndroidKeyEvent.KEYCODE_META_RIGHT -> 0xffeb
        in AndroidKeyEvent.KEYCODE_F1..AndroidKeyEvent.KEYCODE_F12 ->
            0xffbe + (keyCode - AndroidKeyEvent.KEYCODE_F1)
        else -> if (unicodeChar > 0) unicodeChar else 0
    }
}

@Composable
fun DisplayViewportScreen(
    environmentId: String = "",
    displayAddress: String = "127.0.0.1",
    displayPort: Int = 5901,
    managedDesktop: Boolean = true,
    targetWidth: Int = 1280,
    targetHeight: Int = 720,
    audioEnabled: Boolean = true,
    onDisconnect: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val keyboardController = LocalSoftwareKeyboardController.current
    val keyboardFocusRequester = remember { FocusRequester() }

    var cursorPosition by remember { mutableStateOf(Offset(400f, 300f)) }
    var desktopPointer by remember { mutableStateOf(Pair(targetWidth / 2, targetHeight / 2)) }
    var isLeftDown by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf("Initializing display server & RFB stream...") }
    var showToolbar by remember { mutableStateOf(true) }
    var showVirtualKeyboard by remember { mutableStateOf(true) }
    var showSoftInputBox by remember { mutableStateOf(false) }
    var softInputText by remember { mutableStateOf("") }
    var inputMode by remember { mutableStateOf(DesktopInputMode.DirectTouch) }
    var zoomScale by remember { mutableFloatStateOf(1.0f) }
    var muted by remember { mutableStateOf(false) }
    var showExitConfirm by remember { mutableStateOf(false) }
    val displayOperation = remember { Mutex() }
    var desktopSessionId by remember { mutableStateOf<String?>(null) }
    var liveConnection by remember { mutableStateOf(false) }
    var closing by remember { mutableStateOf(false) }

    var ctrlHeld by remember { mutableStateOf(false) }
    var altHeld by remember { mutableStateOf(false) }
    var shiftHeld by remember { mutableStateOf(false) }

    var displayWidth by remember { mutableIntStateOf(targetWidth) }
    var displayHeight by remember { mutableIntStateOf(targetHeight) }
    var measuredFps by remember { mutableIntStateOf(0) }
    var surfaceHolderRef by remember { mutableStateOf<SurfaceHolder?>(null) }
    var surfaceEpoch by remember { mutableIntStateOf(0) }
    val surfaceRevision = remember { AtomicInteger() }
    val canvasSurfaceLock = remember { Any() }
    val surfaceProducer = remember { Mutex() }
    var canvasRenderer by remember { mutableStateOf(false) }

    // Prevent accidental Android Back termination of live desktop session
    BackHandler {
        showExitConfirm = true
    }

    DisposableEffect(Unit) {
        LinuxRuntimeService.start(context, "Desktop Viewport ($displayAddress:$displayPort)")
        if (audioEnabled) {
            AudioBridgePlayer.start(scope, 4713)
        }
        onDispose {
            AudioBridgePlayer.stop(scope)
        }
    }

    fun closeViewport(exitDesktop: Boolean) {
        if (closing) return
        closing = true
        liveConnection = false
        showExitConfirm = false
        scope.launch {
            displayOperation.withLock {
                withContext(Dispatchers.IO) {
                    NativeBridge.detachDisplaySurface()
                    NativeBridge.disconnectDisplayClient()
                }
                val sessionId = desktopSessionId
                if (exitDesktop && sessionId != null) {
                    val result = NativeBridge.stopDesktopSession(sessionId)
                    if (result.isFailure) {
                        statusMessage = "Could not exit desktop: ${result.exceptionOrNull()?.message}"
                        closing = false
                        return@withLock
                    }
                }
                if (!managedDesktop || exitDesktop) {
                    context.stopService(android.content.Intent(context, LinuxRuntimeService::class.java))
                }
                onDisconnect()
            }
        }
    }

    // Start display server if available, connect RFB stream, and run active SurfaceView rendering loop
    LaunchedEffect(environmentId, displayAddress, displayPort, targetWidth, targetHeight) {
        val connected = displayOperation.withLock {
            if (closing) return@withLock false
            if (managedDesktop) {
                statusMessage = "Starting Xvnc + XFCE desktop session..."
                val startRes = NativeBridge.startDesktopSession(
                    envId = environmentId,
                    displayNum = 1,
                    width = targetWidth,
                    height = targetHeight,
                    dpi = 160,
                    port = displayPort
                )
                desktopSessionId = startRes.getOrNull()?.session_id
                if (desktopSessionId == null) {
                    statusMessage = "Desktop error: ${startRes.exceptionOrNull()?.message ?: "missing session ID"}"
                    return@withLock false
                }
            }
            if (closing) return@withLock false
            val (w, h) = NativeBridge.connectDisplayClient(displayAddress, displayPort)
            displayWidth = w.coerceAtLeast(1)
            displayHeight = h.coerceAtLeast(1)
            liveConnection = withContext(Dispatchers.IO) { NativeBridge.hasLiveDisplayConnection() }
            if (!liveConnection) {
                statusMessage = "Could not connect to the display"
                return@withLock false
            }
            val desktopName = if (managedDesktop) "XFCE" else "VNC"
            statusMessage = "Live $desktopName [${displayWidth}x${displayHeight}]"
            true
        }
        if (!connected) return@LaunchedEffect

        while (isActive && !closing) {
            if (!withContext(Dispatchers.IO) { NativeBridge.hasLiveDisplayConnection() }) {
                liveConnection = false
                statusMessage = "Display disconnected. Reopen the viewport to reconnect."
                withContext(Dispatchers.IO) {
                    NativeBridge.detachDisplaySurface()
                    NativeBridge.disconnectDisplayClient()
                }
                break
            }
            val (curW, curH) = NativeBridge.getDisplayDimensions()
            if (curW > 0 && curH > 0 && (curW != displayWidth || curH != displayHeight)) {
                displayWidth = curW
                displayHeight = curH
                statusMessage = "Resized [${displayWidth}x${displayHeight}]"
            }
            if (!canvasRenderer) {
                measuredFps = NativeBridge.getPresentedFps()
                if (NativeBridge.displayPresenterFailed()) canvasRenderer = true
            }

            // Sync guest clipboard when ServerCutText arrives
            NativeBridge.takeDisplayClipboard()?.let { guestClip ->
                val cm = context.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                cm?.setPrimaryClip(ClipData.newPlainText("Linux Desktop", guestClip))
            }
            delay(500)
        }
    }

    LaunchedEffect(surfaceEpoch, surfaceHolderRef, liveConnection, canvasRenderer) {
        val surface = surfaceHolderRef?.surface ?: return@LaunchedEffect
        val revision = surfaceEpoch
        if (!liveConnection) return@LaunchedEffect
        surfaceProducer.withLock {
            if (!surface.isValid) return@withLock
            if (canvasRenderer) {
                measuredFps = 0
                withContext(Dispatchers.Default) {
                    presentWithCanvas(surface, canvasSurfaceLock) { surfaceRevision.get() == revision }
                }
            } else if (NativeBridge.attachDisplaySurface(surface) == 0) {
                try {
                    awaitCancellation()
                } finally {
                    NativeBridge.detachDisplaySurface()
                }
            } else {
                canvasRenderer = true
            }
        }
    }

    fun mapOffsetToGuest(offset: Offset, viewWidth: Float, viewHeight: Float): Pair<Int, Int> {
        val scaledW = viewWidth * zoomScale
        val scaledH = viewHeight * zoomScale
        val left = (viewWidth - scaledW) / 2f
        val top = (viewHeight - scaledH) / 2f
        val gx = (((offset.x - left) / scaledW) * displayWidth).toInt().coerceIn(0, displayWidth - 1)
        val gy = (((offset.y - top) / scaledH) * displayHeight).toInt().coerceIn(0, displayHeight - 1)
        return gx to gy
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
            .onPreviewKeyEvent { keyEvent ->
                val native = keyEvent.nativeKeyEvent
                val sym = androidKeyCodeToX11KeySym(native.keyCode, native.unicodeChar)
                if (sym != 0) {
                    val down = if (keyEvent.type == KeyEventType.KeyDown) 1 else 0
                    NativeBridge.sendKey(down, sym.toLong())
                    true
                } else {
                    false
                }
            }
    ) {
        AndroidView(
            factory = { ctx ->
                SurfaceView(ctx).apply {
                    holder.addCallback(object : SurfaceHolder.Callback {
                        override fun surfaceCreated(holder: SurfaceHolder) {
                            surfaceHolderRef = holder
                            surfaceEpoch = surfaceRevision.incrementAndGet()
                        }
                        override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
                            surfaceHolderRef = holder
                        }
                        override fun surfaceDestroyed(holder: SurfaceHolder) {
                            if (surfaceHolderRef == holder) surfaceHolderRef = null
                            synchronized(canvasSurfaceLock) { surfaceRevision.incrementAndGet() }
                            // Join the native producer before returning the destroyed Surface.
                            NativeBridge.detachDisplaySurface()
                        }
                    })
                }
            },
            modifier = Modifier
                .fillMaxSize()
                // Physical mouse & scroll-wheel handling
                .pointerInput(displayWidth, displayHeight, zoomScale) {
                    awaitPointerEventScope {
                        while (true) {
                            val event = awaitPointerEvent()
                            val change = event.changes.firstOrNull() ?: continue
                            if (event.type == PointerEventType.Move || event.type == PointerEventType.Press || event.type == PointerEventType.Release) {
                                val (gx, gy) = mapOffsetToGuest(change.position, size.width.toFloat(), size.height.toFloat())
                                desktopPointer = gx to gy
                                val mask = when {
                                    event.buttons.isSecondaryPressed -> 4
                                    event.buttons.isTertiaryPressed -> 2
                                    change.pressed -> 1
                                    else -> 0
                                }
                                if (event.type != PointerEventType.Move || change.pressed) {
                                    NativeBridge.sendPointer(mask, gx, gy)
                                }
                            } else if (event.type == PointerEventType.Scroll) {
                                val deltaY = change.scrollDelta.y
                                val btn = if (deltaY < 0f) 8 else 16
                                val (gx, gy) = desktopPointer
                                NativeBridge.sendPointer(btn, gx, gy)
                                NativeBridge.sendPointer(0, gx, gy)
                            }
                        }
                    }
                }
                .pointerInput(displayWidth, displayHeight, inputMode, zoomScale) {
                    detectTapGestures(
                        onTap = { offset ->
                            val (gx, gy) = if (inputMode == DesktopInputMode.DirectTouch) {
                                mapOffsetToGuest(offset, size.width.toFloat(), size.height.toFloat()).also {
                                    cursorPosition = offset
                                    desktopPointer = it
                                }
                            } else {
                                desktopPointer
                            }
                            NativeBridge.sendPointer(1, gx, gy)
                            NativeBridge.sendPointer(0, gx, gy)
                        },
                        onLongPress = { offset ->
                            val (gx, gy) = if (inputMode == DesktopInputMode.DirectTouch) {
                                mapOffsetToGuest(offset, size.width.toFloat(), size.height.toFloat()).also {
                                    cursorPosition = offset
                                    desktopPointer = it
                                }
                            } else {
                                desktopPointer
                            }
                            NativeBridge.sendPointer(4, gx, gy)
                            NativeBridge.sendPointer(0, gx, gy)
                        }
                    )
                }
                .pointerInput(displayWidth, displayHeight, inputMode, zoomScale) {
                    detectDragGestures(
                        onDragStart = { offset ->
                            if (inputMode == DesktopInputMode.DirectTouch) {
                                val (gx, gy) = mapOffsetToGuest(offset, size.width.toFloat(), size.height.toFloat())
                                cursorPosition = offset
                                desktopPointer = gx to gy
                                isLeftDown = true
                                NativeBridge.sendPointer(1, gx, gy)
                            }
                        },
                        onDrag = { change, dragAmount ->
                            change.consume()
                            when (inputMode) {
                                DesktopInputMode.DirectTouch -> {
                                    cursorPosition += dragAmount
                                    val (gx, gy) = mapOffsetToGuest(cursorPosition, size.width.toFloat(), size.height.toFloat())
                                    desktopPointer = gx to gy
                                    NativeBridge.sendPointer(1, gx, gy)
                                }
                                DesktopInputMode.Trackpad, DesktopInputMode.Relative -> {
                                    val gain = if (inputMode == DesktopInputMode.Relative) 1.5f else 1.0f
                                    val nx = (desktopPointer.first + (dragAmount.x * gain).toInt()).coerceIn(0, displayWidth - 1)
                                    val ny = (desktopPointer.second + (dragAmount.y * gain).toInt()).coerceIn(0, displayHeight - 1)
                                    desktopPointer = nx to ny
                                    NativeBridge.sendPointer(0, nx, ny)
                                }
                            }
                        },
                        onDragEnd = {
                            if (inputMode == DesktopInputMode.DirectTouch) {
                                isLeftDown = false
                                val (gx, gy) = desktopPointer
                                NativeBridge.sendPointer(0, gx, gy)
                            }
                        }
                    )
                }
        )

        // Top HUD Header (collapsible for fullscreen mode)
        if (showToolbar) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .statusBarsPadding()
                    .padding(horizontal = 10.dp, vertical = 14.dp)
                    .background(SurfaceDark.copy(alpha = 0.88f), RoundedCornerShape(8.dp))
                    .border(1.dp, CardBorder, RoundedCornerShape(8.dp))
                    .padding(horizontal = 12.dp, vertical = 6.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                    Box(modifier = Modifier.size(8.dp).background(StatusGreen, RoundedCornerShape(4.dp)))
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = if (liveConnection && !canvasRenderer) {
                            "$statusMessage • $measuredFps posts/s"
                        } else statusMessage,
                        style = MaterialTheme.typography.labelSmall,
                        color = TextPrimary,
                        maxLines = 1
                    )
                }

                Row(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.horizontalScroll(rememberScrollState())
                ) {
                    // Input mode cycle
                    OutlinedButton(
                        onClick = {
                            val all = DesktopInputMode.entries
                            inputMode = all[(inputMode.ordinal + 1) % all.size]
                        },
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text(inputMode.label, style = MaterialTheme.typography.labelSmall, color = AccentCyan)
                    }

                    // Resolution cycle (1280x720 -> 1600x900 -> 1920x1080)
                    OutlinedButton(
                        onClick = {
                            val next = when (displayWidth) {
                                1280 -> 1600 to 900
                                1600 -> 1920 to 1080
                                else -> 1280 to 720
                            }
                            scope.launch {
                                NativeBridge.resizeDesktopSession(next.first, next.second)
                            }
                        },
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text("${displayWidth}x${displayHeight}", style = MaterialTheme.typography.labelSmall, color = TextPrimary)
                    }

                    // Zoom / Fit
                    OutlinedButton(
                        onClick = {
                            zoomScale = if (zoomScale < 1.2f) 1.5f else 1.0f
                        },
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text(if (zoomScale > 1.1f) "1.5x" else "Fit", style = MaterialTheme.typography.labelSmall, color = TextPrimary)
                    }

                    // Audio mute toggle
                    if (audioEnabled) {
                        OutlinedButton(
                            onClick = {
                                muted = !muted
                                AudioBridgePlayer.setMuted(muted)
                            },
                            shape = RoundedCornerShape(6.dp),
                            contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                            modifier = Modifier.height(30.dp)
                        ) {
                            Text(if (muted) "Muted" else "Audio", style = MaterialTheme.typography.labelSmall, color = TextPrimary)
                        }
                    }

                    // Sync Android Clipboard -> Guest
                    OutlinedButton(
                        onClick = {
                            val cm = context.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                            val clip = cm?.primaryClip?.getItemAt(0)?.coerceToText(context)?.toString()
                            if (!clip.isNullOrEmpty()) {
                                NativeBridge.sendDisplayClipboard(clip)
                                statusMessage = "Clipboard sent (${clip.length} chars)"
                            }
                        },
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text("Paste", style = MaterialTheme.typography.labelSmall, color = TextPrimary)
                    }

                    // Soft keyboard toggle
                    Button(
                        onClick = {
                            showSoftInputBox = !showSoftInputBox
                            if (showSoftInputBox) {
                                scope.launch {
                                    delay(100)
                                    keyboardFocusRequester.requestFocus()
                                    keyboardController?.show()
                                }
                            } else {
                                keyboardController?.hide()
                            }
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text("IME", style = MaterialTheme.typography.labelSmall, color = AccentCyan)
                    }

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
                        onClick = { canvasRenderer = !canvasRenderer },
                        enabled = liveConnection && !closing,
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text(if (canvasRenderer) "Canvas" else "Native", style = MaterialTheme.typography.labelSmall)
                    }

                    Button(
                        onClick = { showToolbar = false },
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text("Hide", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                    }

                    Button(
                        onClick = { if (managedDesktop) showExitConfirm = true else closeViewport(false) },
                        enabled = !closing,
                        colors = ButtonDefaults.buttonColors(containerColor = StatusRed.copy(alpha = 0.85f)),
                        shape = RoundedCornerShape(6.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Text(if (managedDesktop) "Exit" else "Disconnect", style = MaterialTheme.typography.labelSmall, color = Color.White)
                    }
                }
            }
        } else {
            // Compact HUD restore pill in fullscreen mode
            Button(
                onClick = { showToolbar = true },
                colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark.copy(alpha = 0.65f)),
                shape = RoundedCornerShape(8.dp),
                contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .statusBarsPadding()
                    .padding(14.dp)
                    .height(30.dp)
            ) {
                Text("HUD", style = MaterialTheme.typography.labelSmall, color = AccentCyan)
            }
        }

        // Soft keyboard input receiver bar
        if (showSoftInputBox) {
            Row(
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(top = 56.dp, start = 16.dp, end = 16.dp)
                    .background(SurfaceDark.copy(alpha = 0.94f), RoundedCornerShape(8.dp))
                    .border(1.dp, AccentCyan, RoundedCornerShape(8.dp))
                    .padding(horizontal = 12.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("Type to X11: ", color = TextSecondary, fontSize = 12.sp)
                BasicTextField(
                    value = softInputText,
                    onValueChange = { updated ->
                        if (updated.length > softInputText.length) {
                            val added = updated.substring(softInputText.length)
                            for (ch in added) {
                                val sym = androidCharToX11KeySym(ch)
                                NativeBridge.sendKey(1, sym.toLong())
                                NativeBridge.sendKey(0, sym.toLong())
                            }
                        } else if (updated.length < softInputText.length) {
                            repeat(softInputText.length - updated.length) {
                                NativeBridge.sendKey(1, 0xff08L)
                                NativeBridge.sendKey(0, 0xff08L)
                            }
                        }
                        softInputText = updated.takeLast(32)
                    },
                    textStyle = TextStyle(color = TextPrimary, fontSize = 14.sp),
                    modifier = Modifier
                        .width(200.dp)
                        .focusRequester(keyboardFocusRequester)
                )
            }
        }

        // Floating Virtual Modifier & Function Key Bar
        if (showToolbar && showVirtualKeyboard) {
            val keySymMap = listOf(
                "Esc" to 0xff1bL,
                "Tab" to 0xff09L,
                "Super" to 0xffebL,
                "Enter" to 0xff0dL,
                "Bksp" to 0xff08L,
                "↑" to 0xff52L,
                "↓" to 0xff54L,
                "←" to 0xff51L,
                "→" to 0xff53L,
                "F1" to 0xffbeL,
                "F2" to 0xffbfL,
                "F4" to 0xffc1L,
                "F5" to 0xffc2L,
                "F11" to 0xffc8L
            )

            Row(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .fillMaxWidth()
                    .padding(bottom = 24.dp, start = 8.dp, end = 8.dp)
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
                    colors = ButtonDefaults.buttonColors(containerColor = if (ctrlHeld) AccentCyan else SurfaceContainer),
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
                    colors = ButtonDefaults.buttonColors(containerColor = if (altHeld) AccentCyan else SurfaceContainer),
                    shape = RoundedCornerShape(6.dp),
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                    modifier = Modifier.height(34.dp)
                ) {
                    Text("Alt", style = MaterialTheme.typography.labelSmall, color = if (altHeld) BgDark else AccentCyan, fontWeight = FontWeight.Bold)
                }

                Button(
                    onClick = {
                        shiftHeld = !shiftHeld
                        NativeBridge.sendKey(if (shiftHeld) 1 else 0, 0xffe1L)
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = if (shiftHeld) AccentCyan else SurfaceContainer),
                    shape = RoundedCornerShape(6.dp),
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                    modifier = Modifier.height(34.dp)
                ) {
                    Text("Shift", style = MaterialTheme.typography.labelSmall, color = if (shiftHeld) BgDark else AccentCyan, fontWeight = FontWeight.Bold)
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

        if (showExitConfirm) {
            AlertDialog(
                onDismissRequest = { showExitConfirm = false },
                title = { Text("Close viewport?", color = TextPrimary) },
                text = {
                    if (managedDesktop) Text(
                        "Disconnect to leave the desktop running, or exit to close the desktop and its apps.",
                        color = TextSecondary
                    )
                },
                confirmButton = {
                    TextButton(onClick = { closeViewport(managedDesktop) }) {
                        Text(if (managedDesktop) "Exit Desktop" else "Disconnect")
                    }
                },
                dismissButton = {
                    if (managedDesktop) TextButton(onClick = { closeViewport(false) }) { Text("Disconnect") }
                }
            )
        }
    }
}
