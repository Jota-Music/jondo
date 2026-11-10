package dev.gpui.mobile;

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationChannel;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.media.AudioAttributes;
import android.media.AudioFocusRequest;
import android.media.AudioManager;
import android.os.Build;
import android.support.v4.media.MediaMetadataCompat;
import android.support.v4.media.session.MediaSessionCompat;
import android.support.v4.media.session.PlaybackStateCompat;
import android.util.Log;

import androidx.core.app.NotificationCompat;
import androidx.core.content.ContextCompat;

/**
 * Manages an Android MediaSession for audio/video playback.
 *
 * Provides:
 * - System media notification with playback controls
 * - Lock screen and system panel playback info
 * - Media button handling (headphone buttons, car controls)
 * - Volume key integration
 *
 * All methods are static and called from Rust via JNI.
 */
public class GpuiMediaSession {

    private static final String TAG = "GpuiMediaSession";
    private static final String CHANNEL_ID = "gpui_media_playback";
    private static final int NOTIFICATION_ID = 1001;

    private static String sTitle = "Jondo";
    private static String sArtist = "";
    private static long sDurationMs = 0;
    private static Bitmap sArtwork;
    private static boolean sPlaying;

    private static MediaSessionCompat sSession;
    private static NotificationManager sNotificationManager;
    private static Activity sActivity;

    private static boolean sPauseOnDisconnect = true;
    private static boolean sPauseOnFocus = true;
    private static boolean sFocusHeld;
    private static boolean sPausedByFocus;
    private static boolean sResumeOnGain;
    private static AudioManager sAudioManager;
    private static AudioFocusRequest sFocusRequest;
    private static BroadcastReceiver sNoisyReceiver;
    private static android.os.Handler sCallbackHandler;

    /**
     * Initialize the media session. Call once when playback starts.
     *
     * @param activity The current Activity.
     */
    public static void init(Activity activity) {
        if (sSession != null) return;
        sActivity = activity;
        sAudioManager = (AudioManager) activity.getSystemService(Context.AUDIO_SERVICE);
        try {
        sSession = new MediaSessionCompat(activity, "GpuiMediaSession");
        sSession.setFlags(
                MediaSessionCompat.FLAG_HANDLES_MEDIA_BUTTONS |
                MediaSessionCompat.FLAG_HANDLES_TRANSPORT_CONTROLS
        );

        // The session builds its own Handler, and the Rust side calls this from
        // a native thread without a Looper. Pin the callbacks to the UI thread.
        sCallbackHandler = new android.os.Handler(android.os.Looper.getMainLooper());
        sSession.setCallback(new MediaSessionCompat.Callback() {
            @Override
            public void onPlay() {
                Log.i(TAG, "MediaSession callback: onPlay");
                sPausedByFocus = false;
                sResumeOnGain = false;
                nativeMediaAction("play");
            }

            @Override
            public void onPause() {
                Log.i(TAG, "MediaSession callback: onPause");
                sPausedByFocus = false;
                sResumeOnGain = false;
                abandonFocus();
                nativeMediaAction("pause");
            }

            @Override
            public void onStop() {
                Log.i(TAG, "MediaSession callback: onStop");
                sPausedByFocus = false;
                sResumeOnGain = false;
                abandonFocus();
                nativeMediaAction("stop");
            }

            @Override
            public void onSkipToNext() {
                Log.i(TAG, "MediaSession callback: onSkipToNext");
                nativeMediaAction("next");
            }

            @Override
            public void onSkipToPrevious() {
                Log.i(TAG, "MediaSession callback: onSkipToPrevious");
                nativeMediaAction("previous");
            }

            @Override
            public void onSeekTo(long pos) {
                Log.i(TAG, "MediaSession callback: onSeekTo " + pos);
                nativeMediaSeek(pos);
            }
        }, sCallbackHandler);

        sSession.setActive(true);

        sNoisyReceiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context context, Intent intent) {
                if (sPauseOnDisconnect
                        && sPlaying
                        && AudioManager.ACTION_AUDIO_BECOMING_NOISY.equals(intent.getAction())) {
                    sPausedByFocus = false;
                    sResumeOnGain = false;
                    nativeMediaAction("pause");
                }
            }
        };
        activity.runOnUiThread(() -> {
            try {
                ContextCompat.registerReceiver(
                        activity,
                        sNoisyReceiver,
                        new IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY),
                        ContextCompat.RECEIVER_NOT_EXPORTED
                );
            } catch (Throwable error) {
                Log.e(TAG, "Failed to register the noisy receiver", error);
            }
        });

        // Create notification channel (API 26+)
        sNotificationManager = (NotificationManager) activity.getSystemService(Context.NOTIFICATION_SERVICE);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel channel = new NotificationChannel(
                    CHANNEL_ID,
                    "Media Playback",
                    NotificationManager.IMPORTANCE_LOW
            );
            channel.setDescription("Controls for audio and video playback");
            channel.setShowBadge(false);
            sNotificationManager.createNotificationChannel(channel);
        }

        Log.i(TAG, "MediaSession initialized");
        } catch (Throwable error) {
            Log.e(TAG, "MediaSession init failed", error);
        }
    }

    /**
     * Apply the user's interruption preferences. Disabling focus handling also
     * gives the focus back, so another app can take over without waiting.
     */
    public static void setPreferences(boolean pauseOnDisconnect, boolean pauseOnFocus) {
        sPauseOnDisconnect = pauseOnDisconnect;
        sPauseOnFocus = pauseOnFocus;
        if (!sPauseOnFocus) {
            sPausedByFocus = false;
            sResumeOnGain = false;
            abandonFocus();
        } else if (sPlaying) {
            requestFocus();
        }
    }

    private static boolean requestFocus() {
        if (!sPauseOnFocus || sFocusHeld) {
            return true;
        }
        if (sAudioManager == null || sCallbackHandler == null) {
            return true;
        }

        AudioAttributes attributes = new AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_MEDIA)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                .build();
        sFocusRequest = new AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                .setAudioAttributes(attributes)
                .setOnAudioFocusChangeListener(GpuiMediaSession::onFocusChange, sCallbackHandler)
                .build();
        sFocusHeld = sAudioManager.requestAudioFocus(sFocusRequest)
                == AudioManager.AUDIOFOCUS_REQUEST_GRANTED;
        if (!sFocusHeld) {
            Log.w(TAG, "Audio focus was denied");
        }
        return sFocusHeld;
    }

    private static void abandonFocus() {
        if (sAudioManager != null && sFocusRequest != null && sFocusHeld) {
            sAudioManager.abandonAudioFocusRequest(sFocusRequest);
        }
        sFocusHeld = false;
        sFocusRequest = null;
    }

    private static void onFocusChange(int change) {
        switch (change) {
            case AudioManager.AUDIOFOCUS_LOSS:
                sPausedByFocus = true;
                sResumeOnGain = false;
                abandonFocus();
                if (sPlaying) {
                    nativeMediaAction("pause");
                }
                break;

            case AudioManager.AUDIOFOCUS_LOSS_TRANSIENT:
                sPausedByFocus = true;
                sResumeOnGain = sPlaying;
                if (sPlaying) {
                    nativeMediaAction("pause");
                }
                break;

            case AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK:
                break;

            case AudioManager.AUDIOFOCUS_GAIN: {
                boolean resume = sResumeOnGain;
                sPausedByFocus = false;
                sResumeOnGain = false;
                if (resume) {
                    nativeMediaAction("play");
                }
                break;
            }
        }
    }

    /**
     * Update the media metadata (title, artist, duration).
     *
     * @param title     Track/video title.
     * @param artist    Artist name (or app name).
     * @param durationMs Duration in milliseconds.
     */
    public static void setMetadata(String title, String artist, long durationMs) {
        if (sSession == null) return;

        sTitle = title != null ? title : "Unknown";
        sArtist = artist != null ? artist : "Jondo";
        sDurationMs = durationMs;
        pushMetadata();
    }

    /**
     * Attach the cover already decoded on disk. The notification and the lock
     * screen read it from the metadata bitmap, so it has to be re-published.
     */
    public static void setArtwork(String path) {
        if (sSession == null || path == null) return;
        Bitmap decoded = BitmapFactory.decodeFile(path);
        if (decoded == null) return;
        sArtwork = decoded;
        pushMetadata();
    }

    private static void pushMetadata() {
        if (sSession == null) return;
        MediaMetadataCompat.Builder builder = new MediaMetadataCompat.Builder()
                .putString(MediaMetadataCompat.METADATA_KEY_TITLE, sTitle)
                .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, sArtist)
                .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, sDurationMs);
        if (sArtwork != null) {
            builder.putBitmap(MediaMetadataCompat.METADATA_KEY_ALBUM_ART, sArtwork);
            builder.putBitmap(MediaMetadataCompat.METADATA_KEY_ART, sArtwork);
        }
        sSession.setMetadata(builder.build());
        updateNotification(sPlaying);
        Log.i(TAG, "Metadata updated: " + sTitle + " by " + sArtist + " (" + sDurationMs + "ms) art=" + (sArtwork != null));
    }

    /**
     * Update the playback state.
     *
     * @param isPlaying  Whether playback is active.
     * @param positionMs Current playback position in milliseconds.
     * @param speed      Playback speed (1.0 = normal).
     */
    public static void setPlaybackState(boolean isPlaying, long positionMs, float speed) {
        if (sSession == null) return;

        int state = isPlaying
                ? PlaybackStateCompat.STATE_PLAYING
                : PlaybackStateCompat.STATE_PAUSED;

        long actions = PlaybackStateCompat.ACTION_PLAY
                | PlaybackStateCompat.ACTION_PAUSE
                | PlaybackStateCompat.ACTION_PLAY_PAUSE
                | PlaybackStateCompat.ACTION_STOP
                | PlaybackStateCompat.ACTION_SEEK_TO
                | PlaybackStateCompat.ACTION_SKIP_TO_NEXT
                | PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS;

        PlaybackStateCompat playbackState = new PlaybackStateCompat.Builder()
                .setActions(actions)
                .setState(state, positionMs, speed)
                .build();

        sPlaying = isPlaying;
        if (isPlaying) {
            sPausedByFocus = false;
            if (!requestFocus()) {
                sPlaying = false;
                nativeMediaAction("pause");
                return;
            }
        } else if (!sPausedByFocus) {
            sResumeOnGain = false;
            abandonFocus();
        }
        sSession.setPlaybackState(playbackState);
        updateNotification(isPlaying);
    }

    /**
     * Show or update the media notification with playback controls.
     */
    private static void updateNotification(boolean isPlaying) {
        if (sActivity == null || sSession == null || sNotificationManager == null) return;

        try {
            // Intent to reopen the app when notification is tapped
            Intent openIntent = sActivity.getPackageManager()
                    .getLaunchIntentForPackage(sActivity.getPackageName());
            PendingIntent contentIntent = PendingIntent.getActivity(
                    sActivity, 0, openIntent,
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
            );

            MediaMetadataCompat metadata = sSession.getController().getMetadata();
            String title = metadata != null
                    ? metadata.getString(MediaMetadataCompat.METADATA_KEY_TITLE)
                    : "GPUI";
            String artist = metadata != null
                    ? metadata.getString(MediaMetadataCompat.METADATA_KEY_ARTIST)
                    : "";

            // Use MediaStyle for system panel integration
            androidx.media.app.NotificationCompat.MediaStyle mediaStyle =
                    new androidx.media.app.NotificationCompat.MediaStyle()
                            .setMediaSession(sSession.getSessionToken())
                            .setShowActionsInCompactView(0, 1, 2);

            NotificationCompat.Builder builder = new NotificationCompat.Builder(sActivity, CHANNEL_ID)
                    .setContentTitle(title)
                    .setContentText(artist)
                    .setSmallIcon(android.R.drawable.ic_media_play)
                    .setLargeIcon(sArtwork)
                    .setContentIntent(contentIntent)
                    .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
                    .setOngoing(isPlaying)
                    .setStyle(mediaStyle)
                    // Previous
                    .addAction(new NotificationCompat.Action(
                            android.R.drawable.ic_media_previous,
                            "Previous",
                            actionIntent("previous")
                    ))
                    // Play / Pause
                    .addAction(new NotificationCompat.Action(
                            isPlaying ? android.R.drawable.ic_media_pause : android.R.drawable.ic_media_play,
                            isPlaying ? "Pause" : "Play",
                            actionIntent(isPlaying ? "pause" : "play")
                    ))
                    // Next
                    .addAction(new NotificationCompat.Action(
                            android.R.drawable.ic_media_next,
                            "Next",
                            actionIntent("next")
                    ));

            sNotificationManager.notify(NOTIFICATION_ID, builder.build());
        } catch (Exception e) {
            Log.e(TAG, "Failed to update notification", e);
        }
    }

    /**
     * A notification button that reports the action through {@link GpuiMediaAction}.
     */
    private static PendingIntent actionIntent(String action) {
        Intent intent = new Intent(sActivity, GpuiMediaAction.class);
        intent.putExtra(GpuiMediaAction.EXTRA, action);
        return PendingIntent.getBroadcast(
                sActivity,
                action.hashCode(),
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
        );
    }

    /**
     * Called by {@link GpuiMediaAction} when a notification button is pressed.
     */
    static void dispatch(String action) {
        if (sSession == null) return;
        nativeMediaAction(action);
    }

    /**
     * Release the media session and dismiss the notification.
     */
    public static void release() {
        abandonFocus();
        if (sNoisyReceiver != null && sActivity != null) {
            try {
                sActivity.unregisterReceiver(sNoisyReceiver);
            } catch (Exception error) {
                Log.w(TAG, "Failed to unregister the noisy receiver", error);
            }
        }
        sNoisyReceiver = null;
        if (sSession != null) {
            sSession.setActive(false);
            sSession.release();
            sSession = null;
        }
        if (sNotificationManager != null) {
            sNotificationManager.cancel(NOTIFICATION_ID);
            sNotificationManager = null;
        }
        sPlaying = false;
        sPausedByFocus = false;
        sResumeOnGain = false;
        sAudioManager = null;
        sCallbackHandler = null;
        sActivity = null;
        Log.i(TAG, "MediaSession released");
    }

    /**
     * Get the MediaSessionCompat token for volume routing.
     * Returns null if not initialized.
     */
    public static MediaSessionCompat.Token getSessionToken() {
        return sSession != null ? sSession.getSessionToken() : null;
    }

    /**
     * JNI callback: notify Rust of a media action from system controls.
     * Actions: "play", "pause", "stop", "next", "previous"
     */
    private static native void nativeMediaAction(String action);

    /**
     * JNI callback: notify Rust of a seek request from system controls.
     */
    private static native void nativeMediaSeek(long positionMs);
}
