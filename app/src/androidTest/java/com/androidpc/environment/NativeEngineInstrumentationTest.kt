package com.androidpc.environment

import android.graphics.PixelFormat
import android.media.ImageReader
import android.os.SystemClock

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.bridge.downloadRootfs
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.DataInputStream
import java.net.ServerSocket
import java.nio.ByteBuffer
import java.util.UUID
import org.json.JSONObject

@RunWith(AndroidJUnit4::class)
class NativeEngineInstrumentationTest {

    @Test
    fun installsBothArm64GuestsAndRunsAResizedInteractiveTerminal() = runBlocking {
        val arguments = InstrumentationRegistry.getArguments()
        assumeTrue(arguments.getString("rootfsAcceptance") == "true")
        val exerciseNetwork = arguments.getString("rootfsNetwork") == "true"
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        NativeBridge.initEngine(context).getOrThrow()

        for (distro in listOf("ubuntu", "debian")) {
            val fixture = File(context.cacheDir, "acceptance-$distro.part")
            check(fixture.isFile) { "Missing verified acceptance fixture: " + fixture.name }
            val archive = if (exerciseNetwork) File.createTempFile("resume-$distro-", ".part", context.cacheDir) else fixture
            val envId = UUID.randomUUID().toString()
            val sessionId = "terminal-$envId"
            var installed = false
            var terminalOpen = false
            try {
                val meta = NativeBridge.getArtifactInfo(distro).getOrThrow()
                if (arguments.getString("signedCatalog") == "true") {
                    assertFalse("The signed catalog should replace the debug artifact name", meta.name.endsWith("-arm64"))
                }
                if (exerciseNetwork) {
                    val prefix = ByteArray(65_536)
                    DataInputStream(fixture.inputStream()).use { it.readFully(prefix) }
                    archive.writeBytes(prefix)
                    downloadRootfs(meta, archive, context.filesDir) {}
                    assertEquals(meta.size_bytes, archive.length())
                }
                val corrupt = File.createTempFile("corrupt-rootfs-", ".part", context.cacheDir)
                try {
                    assertTrue(NativeBridge.installEnvironment(envId, distro, 2048, corrupt, "Acceptance $distro").isFailure)
                    assertTrue(NativeBridge.listEnvironments().getOrThrow().none { it.id == envId })
                } finally {
                    corrupt.delete()
                }

                NativeBridge.installEnvironment(envId, distro, 2048, archive, "Acceptance $distro").getOrThrow()
                installed = true
                assertTrue(NativeBridge.listEnvironments().getOrThrow().any { it.id == envId })
                assertTrue(NativeBridge.installEnvironment(envId, distro, 2048, archive, "Acceptance $distro").isFailure)
                NativeBridge.createTerminalForEnv(sessionId, envId).getOrThrow()
                terminalOpen = true
                NativeBridge.resizeTerminal(sessionId, 96, 34).getOrThrow()
                val command = "uname -m; cat /etc/os-release; apt --version; stty size; printf 'ROOTFS_%s\\n' DONE\n"
                assertEquals(command.toByteArray().size, NativeBridge.writeTerminal(sessionId, command.toByteArray()).getOrThrow())
                val output = StringBuilder()
                val lines = withTimeout(20_000) {
                    while (true) {
                        output.append(NativeBridge.readTerminal(sessionId).getOrThrow().decodeToString())
                        if (output.contains("ROOTFS_DONE")) break
                        delay(25)
                    }
                    output.toString().lineSequence().map { it.trim() }.toList()
                }
                assertTrue(output.toString(), lines.contains("aarch64"))
                assertTrue(output.toString(), lines.any { it == "ID=$distro" || it == "ID=\"$distro\"" })
                assertTrue(output.toString(), lines.any { it.startsWith("apt ") && it.contains("(arm64)") })
                assertTrue(output.toString(), lines.contains("34 96"))
                assertEquals("Running", NativeBridge.getEnvironmentState(envId))
                NativeBridge.writeTerminal(sessionId, "exit 7\n".toByteArray()).getOrThrow()
                val exitStatus = withTimeout(10_000) {
                    var status = NativeBridge.terminalExitStatus(sessionId)
                    while (status == -1) {
                        delay(25)
                        status = NativeBridge.terminalExitStatus(sessionId)
                    }
                    status
                }
                assertEquals(7, exitStatus)
                assertEquals("Stopped", NativeBridge.getEnvironmentState(envId))
                NativeBridge.recoverEnvironment(envId, restart = false).getOrThrow()
                assertEquals("Stopped", NativeBridge.getEnvironmentState(envId))
                if (arguments.getString("stage31Apt") == "true") {
                    verifyStage31Sources(context.filesDir, envId, distro)
                }
                println("ROOTFS_ACCEPTANCE " + lines.filter {
                    it == "aarch64" || it.startsWith("PRETTY_NAME=") || it.startsWith("apt ") || it == "34 96"
                }.joinToString(" | "))
            } finally {
                if (terminalOpen) NativeBridge.closeTerminal(sessionId).getOrThrow()
                if (installed) NativeBridge.deleteEnvironment(envId).getOrThrow()
                if (exerciseNetwork) archive.delete()
            }
            assertTrue(NativeBridge.listEnvironments().getOrThrow().none { it.id == envId })
        }
    }

    private suspend fun verifyStage31Sources(filesDir: File, envId: String, distro: String) {
        val apt = File(filesDir, "environments/$distro/$envId/rootfs/etc/apt")
        if (distro == "debian") {
            val source = File(apt, "sources.list.d/debian.sources")
            val before = source.readBytes()
            NativeBridge.recoverEnvironment(envId, restart = false).getOrThrow()
            assertArrayEquals(before, source.readBytes())
            assertFalse(File(apt, "sources.list.d/debian.sources.androidpc-original").exists())
            return
        }

        val source = File(apt, "sources.list.d/ubuntu.sources")
        val ready = source.readText()
        val ports = "https://ports.ubuntu.com/ubuntu-ports/"
        assertTrue(ready.contains(ports))
        assertTrue(ready.contains("resolute-security"))
        val backup = File(source.parentFile, "ubuntu.sources.androidpc-original")
        val original = backup.readText()
        val faulty = ready.replaceFirst(ports, "http://archive.ubuntu.com/ubuntu/")
            .replace(ports, "http://security.ubuntu.com/ubuntu/")
        source.writeText(faulty)
        repeat(2) {
            NativeBridge.recoverEnvironment(envId, restart = false).getOrThrow()
            assertEquals(ready, source.readText())
            assertEquals(original, backup.readText())
        }
        source.writeText("Types: deb\nURIs: http://archive.ubuntu.com/ubuntu/\n")
        val malformed = source.readText()
        assertTrue(NativeBridge.recoverEnvironment(envId, restart = false).isFailure)
        assertEquals(malformed, source.readText())
        assertTrue(source.delete())
        val legacy = File(apt, "sources.list")
        val savedLegacy = legacy.takeIf { it.isFile }?.readBytes()
        legacy.delete()
        assertTrue(NativeBridge.recoverEnvironment(envId, restart = false).isFailure)
        if (savedLegacy != null) legacy.writeBytes(savedLegacy)
        source.writeText(ready)
        NativeBridge.recoverEnvironment(envId, restart = false).getOrThrow()

        val session = "stage31-apt-$envId"
        NativeBridge.createTerminalForEnv(session, envId).getOrThrow()
        try {
            val commands = listOf(
                "dpkg --print-architecture",
                "cat /etc/os-release; grep '^URIs:' /etc/apt/sources.list.d/ubuntu.sources",
                "if apt-get -o Acquire::https::Proxy=http://127.0.0.1:1 -o Acquire::Retries=0 -o APT::Update::Error-Mode=any update; then exit 30; fi",
                "apt-get -o APT::Update::Error-Mode=any update",
                "apt-cache policy",
                "DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends hello",
                "dpkg-query -W hello"
            )
            val command = commands.joinToString(" && ") + "; s=\$?; printf 'STAGE31_APT_%s=%s\\n' DONE \"\$s\"\n"
            NativeBridge.writeTerminal(session, command.toByteArray()).getOrThrow()
            val output = withTimeout(240_000) {
                val text = StringBuilder()
                while (!text.contains("STAGE31_APT_DONE=")) {
                    text.append(NativeBridge.readTerminal(session).getOrThrow().decodeToString())
                    delay(80)
                }
                text.toString()
            }
            assertTrue(output, output.contains(Regex("STAGE31_APT_DONE=0\\b")))
            assertTrue(output, output.lineSequence().any { it.trim() == "arm64" })
            assertTrue(output, output.contains("resolute-security"))
        } finally {
            NativeBridge.closeTerminal(session).getOrThrow()
        }
    }

    @Test
    fun desktopExitKeepsItsEnvironmentAndTerminalRunning() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val envId = InstrumentationRegistry.getArguments().getString("stage31DesktopEnv")
        assumeTrue("Requires an isolated environment with Xvnc and XFCE", !envId.isNullOrBlank())
        requireNotNull(envId)
        NativeBridge.initEngine(instrumentation.targetContext).getOrThrow()
        val terminal = "stage31-display-$envId"
        NativeBridge.createTerminalForEnv(terminal, envId).getOrThrow()
        try {
            ServerSocket(0, 1, java.net.InetAddress.getByName("127.0.0.1")).use { occupied ->
                assertTrue(NativeBridge.startDesktopSession(envId, port = occupied.localPort).isFailure)
            }
            var previousId: String? = null
            repeat(3) { iteration ->
                val endpoint = NativeBridge.startDesktopSession(envId).getOrThrow()
                val id = requireNotNull(endpoint.session_id)
                assertNotEquals(previousId, id)
                previousId = id
                try {
                    val identities = withTimeout(15_000) {
                        while (true) {
                            val status = JSONObject(NativeBridge.getDesktopStatus(envId))
                            assertEquals(id, status.getString("session_id"))
                            if (!status.isNull("xvnc_identity") && !status.isNull("xfce_identity")) {
                                return@withTimeout listOf(
                                    status.getString("xvnc_identity"), status.getString("xfce_identity")
                                )
                            }
                            delay(100)
                        }
                        error("Desktop identities unavailable")
                    }
                    assertTrue(identities.all(::isLiveProcess))
                    NativeBridge.connectDisplayClient(port = endpoint.port)
                    assertTrue(NativeBridge.hasLiveDisplayConnection())
                    NativeBridge.disconnectDisplayClient()
                    assertFalse(NativeBridge.hasLiveDisplayConnection())
                    assertTrue(identities.all(::isLiveProcess))
                    assertEquals(id, NativeBridge.startDesktopSession(envId).getOrThrow().session_id)
                    NativeBridge.connectDisplayClient(port = endpoint.port)
                    if (iteration == 2) {
                        NativeBridge.stopEnvironment(envId).getOrThrow()
                    } else {
                        NativeBridge.stopDesktopSession(id).getOrThrow()
                        NativeBridge.stopDesktopSession(id).getOrThrow()
                        assertEquals("Running", NativeBridge.getEnvironmentState(envId))
                        assertEquals(-1, NativeBridge.terminalExitStatus(terminal))
                    }
                    assertFalse(NativeBridge.hasLiveDisplayConnection())
                    assertTrue(identities.none(::isLiveProcess))
                    assertFalse(JSONObject(NativeBridge.getDesktopStatus(envId)).getBoolean("active"))
                } finally {
                    NativeBridge.stopDesktopSession(id).getOrThrow()
                }
            }
        } finally {
            NativeBridge.closeTerminal(terminal)
        }
    }

    private fun isLiveProcess(identity: String): Boolean {
        val (pid, started) = identity.split(' ')
        val stat = runCatching { File("/proc/$pid/stat").readText() }.getOrNull() ?: return false
        val fields = stat.substringAfterLast(") ").split(' ')
        return fields.firstOrNull() != "Z" && fields.getOrNull(19) == started
    }

    @Test
    fun nativePresenterRecreatesSurfaceAndTracksGuestResize() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val envId = InstrumentationRegistry.getArguments().getString("stage32RendererEnv")
        assumeTrue("Requires an isolated environment with Xvnc and XFCE", !envId.isNullOrBlank())
        requireNotNull(envId)
        NativeBridge.initEngine(instrumentation.targetContext).getOrThrow()
        val started = SystemClock.elapsedRealtime()
        val endpoint = NativeBridge.startDesktopSession(envId, width = 1280, height = 720).getOrThrow()
        val serverReady = SystemClock.elapsedRealtime()
        val sessionId = requireNotNull(endpoint.session_id)
        try {
            assertEquals(1280 to 720, NativeBridge.connectDisplayClient(port = endpoint.port))
            suspend fun receiveFrame(width: Int, height: Int) {
                ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 3).use { reader ->
                    assertEquals(0, NativeBridge.attachDisplaySurface(reader.surface))
                    try {
                        withTimeout(15_000) {
                            while (true) {
                                assertFalse("The native window failed", NativeBridge.displayPresenterFailed())
                                val image = reader.acquireLatestImage()
                                if (image != null) {
                                    image.use {
                                        assertEquals(width, it.width)
                                        assertEquals(height, it.height)
                                        assertTrue(it.planes[0].buffer.remaining() >= width * height * 4)
                                    }
                                    break
                                }
                                delay(25)
                            }
                        }
                    } finally {
                        NativeBridge.detachDisplaySurface()
                    }
                }
            }
            receiveFrame(1280, 720)
            println("STAGE32_STARTUP server-ready-ms=${serverReady - started} first-image-ms=${SystemClock.elapsedRealtime() - started}")
            receiveFrame(1280, 720)
            assertEquals(1600 to 900, NativeBridge.resizeDesktopSession(1600, 900))
            val pixels = ByteBuffer.allocateDirect(1600 * 900 * 4)
            withTimeout(12_000) {
                while (NativeBridge.getDisplayDimensions() != (1600 to 900)) {
                    val generation = NativeBridge.getFramebufferDirect(pixels, 0)
                    assertTrue(generation >= 0)
                    delay(25)
                }
            }
            receiveFrame(1600, 900)
            assertEquals(sessionId, JSONObject(NativeBridge.getDesktopStatus(envId)).getString("session_id"))
        } finally {
            NativeBridge.detachDisplaySurface()
            NativeBridge.disconnectDisplayClient()
            NativeBridge.stopDesktopSession(sessionId).getOrThrow()
        }
    }

    @Test
    fun rustEngineInitializesAndReportsHealthOnDevice() = runBlocking {
        val appContext = InstrumentationRegistry.getInstrumentation().targetContext
        assertTrue("NativeBridge should load libandroidpc_engine.so", NativeBridge.isLoaded)

        val initRes = NativeBridge.initEngine(appContext)
        assertTrue("Engine init should succeed", initRes.isSuccess)

        val version = NativeBridge.getEngineVersion()
        assertFalse("Engine version should not be empty", version.isBlank())

        val statusRes = NativeBridge.getEngineStatus()
        assertTrue("Engine status should succeed", statusRes.isSuccess)
        assertEquals("Ready", statusRes.getOrNull()?.state)

        val healthRes = NativeBridge.runHealthCheck()
        assertTrue("Health check should succeed", healthRes.isSuccess)

        val diagRes = NativeBridge.getDiagnostics()
        assertTrue("Diagnostics should succeed", diagRes.isSuccess)
        assertEquals("android", diagRes.getOrNull()?.target_os)

        val kvmRes = NativeBridge.probeKvmSupport()
        assertTrue("KVM probe should return structured capability", kvmRes.isSuccess)

        val artifactsRes = NativeBridge.listArtifacts()
        assertTrue("Artifact catalog should list host artifacts", artifactsRes.isSuccess)
        assertTrue("Artifact catalog should be non-empty", (artifactsRes.getOrNull()?.size ?: 0) > 0)
    }
}
