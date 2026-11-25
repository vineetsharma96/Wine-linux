package com.androidpc.environment.service

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.util.Log
import com.androidpc.environment.bridge.NativeBridge
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/**
 * Streams S16LE 44.1kHz stereo PCM frames from the Rust AudioTcpSink (127.0.0.1:4713)
 * into an Android AudioTrack.
 */
object AudioBridgePlayer {
    private const val TAG = "AudioBridgePlayer"
    private const val SAMPLE_RATE = 44100

    private var playbackJob: Job? = null
    private var audioTrack: AudioTrack? = null
    @Volatile var isMuted: Boolean = false
        private set

    fun setMuted(muted: Boolean) {
        isMuted = muted
        try {
            audioTrack?.setVolume(if (muted) 0.0f else 1.0f)
        } catch (_: Exception) {}
    }

    val isActive: Boolean
        get() = playbackJob?.isActive == true

    fun start(scope: CoroutineScope, port: Int = 4713) {
        if (isActive) return
        playbackJob = scope.launch(Dispatchers.IO) {
            val sinkResult = NativeBridge.startAudioSink(port)
            if (sinkResult.isFailure) {
                Log.w(TAG, "Audio sink failed to bind: ${sinkResult.exceptionOrNull()?.message}")
                return@launch
            }

            val minBuf = AudioTrack.getMinBufferSize(
                SAMPLE_RATE,
                AudioFormat.CHANNEL_OUT_STEREO,
                AudioFormat.ENCODING_PCM_16BIT
            ).coerceAtLeast(4096)

            val track = AudioTrack.Builder()
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build()
                )
                .setAudioFormat(
                    AudioFormat.Builder()
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setSampleRate(SAMPLE_RATE)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                        .build()
                )
                .setBufferSizeInBytes(minBuf * 2)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .build()

            audioTrack = track
            track.play()
            val pcmBuf = ByteArray(4096)

            try {
                while (isActive) {
                    val read = NativeBridge.readAudioPcm(pcmBuf)
                    if (read > 0) {
                        track.write(pcmBuf, 0, read)
                    } else {
                        delay(15)
                    }
                }
            } finally {
                try {
                    track.stop()
                    track.release()
                } catch (_: Exception) {}
                audioTrack = null
            }
        }
    }

    fun stop(scope: CoroutineScope) {
        playbackJob?.cancel()
        playbackJob = null
        scope.launch(Dispatchers.IO) {
            NativeBridge.stopAudioSink()
        }
    }
}
