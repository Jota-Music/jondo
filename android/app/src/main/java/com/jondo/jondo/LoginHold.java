package com.jondo.jondo;

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;

/**
 * Keeps the process out of the cached state while a Spotify login is out in the
 * system browser.
 *
 * <p>The OAuth redirect lands on a loopback listener inside this process, so a
 * frozen app never answers it and the login dies at the timeout. This service
 * is started before the browser opens and stopped when the wait ends; the timer
 * is the backstop for the paths that never get there.</p>
 */
public class LoginHold extends Service {

    private static final String CHANNEL = "login";
    private static final int NOTIFICATION = 0x4a4f;
    private static final long LIMIT = 6 * 60 * 1000L;

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Runnable stop = this::stopSelf;

    public static void start(Activity activity) {
        activity.startForegroundService(new Intent(activity, LoginHold.class));
    }

    public static void stop(Activity activity) {
        activity.stopService(new Intent(activity, LoginHold.class));
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        NotificationManager manager = getSystemService(NotificationManager.class);
        if (manager != null) {
            NotificationChannel channel = new NotificationChannel(
                    CHANNEL, "Sign-in", NotificationManager.IMPORTANCE_LOW);
            channel.setShowBadge(false);
            manager.createNotificationChannel(channel);
        }

        Notification notification = new Notification.Builder(this, CHANNEL)
                .setSmallIcon(android.R.drawable.stat_sys_download)
                .setContentTitle("Jondo")
                .setContentText("Signing in to Spotify…")
                .setOngoing(true)
                .build();

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION, notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);
        } else {
            startForeground(NOTIFICATION, notification);
        }

        handler.removeCallbacks(stop);
        handler.postDelayed(stop, LIMIT);
        return START_NOT_STICKY;
    }

    @Override
    public void onDestroy() {
        handler.removeCallbacks(stop);
        stopForeground(STOP_FOREGROUND_REMOVE);
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
