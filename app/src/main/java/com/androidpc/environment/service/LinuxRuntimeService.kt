package com.androidpc.environment.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.androidpc.environment.MainActivity
import com.androidpc.environment.bridge.NativeBridge
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * Foreground service keeping the rootless PRoot/Xvnc/Wine runtime and Audio sink alive
 * while the app is backgrounded.
 */
class LinuxRuntimeService : Service() {
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            AudioBridgePlayer.stop(serviceScope)
            serviceScope.launch {
                NativeBridge.stopDisplayServer()
                stopSelf()
            }
            return START_NOT_STICKY
        }

        ensureNotificationChannel()
        val notification = buildNotification(
            intent?.getStringExtra(EXTRA_STATUS) ?: "Rootless Linux & Wine Runtime Active"
        )

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }

        return START_STICKY
    }

    override fun onDestroy() {
        AudioBridgePlayer.stop(serviceScope)
        serviceScope.cancel()
        super.onDestroy()
    }

    private fun ensureNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Android PC Runtime",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Keeps PRoot Linux, Xvnc display, and Wine sessions running"
            }
            val nm = getSystemService(NotificationManager::class.java)
            nm?.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(statusText: String): Notification {
        val openIntent = Intent(this, MainActivity::class.java)
        val openPending = PendingIntent.getActivity(
            this,
            0,
            openIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val stopIntent = Intent(this, LinuxRuntimeService::class.java).apply {
            action = ACTION_STOP
        }
        val stopPending = PendingIntent.getService(
            this,
            1,
            stopIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Android PC Environment")
            .setContentText(statusText)
            .setSmallIcon(android.R.drawable.sym_def_app_icon)
            .setOngoing(true)
            .setContentIntent(openPending)
            .addAction(android.R.drawable.ic_media_pause, "Stop Runtime", stopPending)
            .build()
    }

    companion object {
        private const val CHANNEL_ID = "androidpc_runtime_channel"
        private const val NOTIFICATION_ID = 1001
        const val ACTION_START = "com.androidpc.environment.action.START_RUNTIME"
        const val ACTION_STOP = "com.androidpc.environment.action.STOP_RUNTIME"
        const val EXTRA_STATUS = "status"

        fun start(context: Context, status: String = "Rootless Linux & Wine Runtime Active") {
            val intent = Intent(context, LinuxRuntimeService::class.java).apply {
                action = ACTION_START
                putExtra(EXTRA_STATUS, status)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            val intent = Intent(context, LinuxRuntimeService::class.java).apply {
                action = ACTION_STOP
            }
            context.startService(intent)
        }
    }
}
