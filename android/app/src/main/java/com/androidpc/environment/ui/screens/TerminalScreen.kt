package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.withContext
import java.util.UUID

@Composable
fun TerminalScreen(
    envId: String,
    defaultShell: String = "/bin/bash",
    onBack: () -> Unit
) {
    val sessionId = remember(envId) { "term-" + UUID.randomUUID() }
    var terminalOutput by remember { mutableStateOf("") }
    var commandInput by remember { mutableStateOf("") }
    var ptyFd by remember { mutableIntStateOf(-1) }
    var viewportSize by remember { mutableStateOf(IntSize.Zero) }
    val density = LocalDensity.current
    val glyphSize = remember(density) {
        android.graphics.Paint().apply {
            typeface = android.graphics.Typeface.MONOSPACE
            textSize = with(density) { 12.sp.toPx() }
        }.let { paint -> paint.measureText("M") to paint.fontSpacing }
    }
    val scrollState = rememberScrollState()
    val scope = rememberCoroutineScope()

    LaunchedEffect(ptyFd, viewportSize, glyphSize) {
        if (ptyFd >= 0 && viewportSize.width > 0 && viewportSize.height > 0) {
            val padding = with(density) { 24.dp.toPx() }
            val cols = ((viewportSize.width - padding) / glyphSize.first).toInt().coerceIn(1, 512)
            val rows = ((viewportSize.height - padding) / glyphSize.second).toInt().coerceIn(1, 512)
            NativeBridge.resizeTerminal(sessionId, cols, rows)
        }
    }

    LaunchedEffect(sessionId) {
        try {
            val createRes = NativeBridge.createTerminalForEnv(sessionId, envId, 80, 24, defaultShell)
            ptyFd = createRes.getOrElse { err ->
                terminalOutput = "Failed to open Linux terminal: ${err.message}\n"
                return@LaunchedEffect
            }
            while (isActive) {
                val read = NativeBridge.readTerminal(sessionId, 4096)
                if (read.isFailure) {
                    terminalOutput += "\nTerminal read failed: ${read.exceptionOrNull()?.message}\n"
                    break
                }
                val bytes = read.getOrThrow()
                if (bytes.isNotEmpty()) {
                    terminalOutput = (terminalOutput + bytes.decodeToString()).takeLast(16384)
                } else {
                    val exit = NativeBridge.terminalExitStatus(sessionId)
                    if (exit != -1) {
                        terminalOutput += if (exit >= 0) "\nShell exited with status $exit\n" else "\nTerminal stopped\n"
                        break
                    }
                }
                delay(60)
            }
        } finally {
            ptyFd = -1
            withContext(NonCancellable) { NativeBridge.closeTerminal(sessionId) }
        }
    }

    LaunchedEffect(terminalOutput) {
        scrollState.animateScrollTo(scrollState.maxValue)
    }

    val sendText: (String) -> Unit = { text ->
        scope.launch {
            NativeBridge.writeTerminal(sessionId, text.toByteArray(Charsets.UTF_8))
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(12.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                TextButton(onClick = onBack) {
                    Text("← Back", color = AccentCyan)
                }
                Spacer(modifier = Modifier.width(4.dp))
                Text(
                    text = "PTY Terminal",
                    style = MaterialTheme.typography.titleMedium,
                    color = TextPrimary,
                    fontWeight = FontWeight.Bold
                )
            }
            Surface(
                shape = RoundedCornerShape(6.dp),
                color = if (ptyFd >= 0) StatusGreen.copy(alpha = 0.15f) else StatusAmber.copy(alpha = 0.15f)
            ) {
                Text(
                    text = if (ptyFd >= 0) "FD $ptyFd" else "OFFLINE",
                    color = if (ptyFd >= 0) StatusGreen else StatusAmber,
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                )
            }
        }

        Spacer(modifier = Modifier.height(8.dp))

        // Terminal Scrollback Buffer
        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .onSizeChanged { viewportSize = it }
                .background(SurfaceDark, RoundedCornerShape(10.dp))
                .border(1.dp, CardBorder, RoundedCornerShape(10.dp))
                .padding(12.dp)
                .verticalScroll(scrollState)
        ) {
            Text(
                text = terminalOutput,
                color = TextPrimary,
                fontFamily = FontFamily.Monospace,
                fontSize = 12.sp,
                lineHeight = 16.sp
            )
        }

        Spacer(modifier = Modifier.height(8.dp))

        // Quick Control Keys
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(6.dp)
        ) {
            val quickKeys = listOf(
                "Ctrl+C" to "\u0003",
                "Tab" to "\t",
                "Esc" to "\u001b",
                "uname -m" to "uname -m\n",
                "env" to "env\n"
            )
            quickKeys.forEach { (label, payload) ->
                OutlinedButton(
                    onClick = { sendText(payload) },
                    shape = RoundedCornerShape(6.dp),
                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                    modifier = Modifier.height(32.dp)
                ) {
                    Text(label, style = MaterialTheme.typography.labelSmall, color = AccentCyan)
                }
            }
            OutlinedButton(
                onClick = { terminalOutput = "" },
                shape = RoundedCornerShape(6.dp),
                contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                modifier = Modifier.height(32.dp)
            ) {
                Text("Clear", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
            }
        }

        Spacer(modifier = Modifier.height(8.dp))

        // Command Input Row
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            OutlinedTextField(
                value = commandInput,
                onValueChange = { commandInput = it },
                modifier = Modifier.weight(1f),
                placeholder = { Text("Enter shell input...", color = TextMuted) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                keyboardActions = KeyboardActions(onSend = {
                    sendText(commandInput + "\n")
                    commandInput = ""
                }),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedTextColor = TextPrimary,
                    unfocusedTextColor = TextPrimary,
                    focusedBorderColor = AccentCyan,
                    unfocusedBorderColor = CardBorder
                )
            )
            Button(
                onClick = {
                    sendText(commandInput + "\n")
                    commandInput = ""
                },
                colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                shape = RoundedCornerShape(8.dp),
                modifier = Modifier.height(52.dp)
            ) {
                Text("Send", color = BgDark, fontWeight = FontWeight.Bold)
            }
        }
    }
}
