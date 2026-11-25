package com.androidpc.environment

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.androidpc.environment.bridge.NativeBridge
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeEngineInstrumentationTest {

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
